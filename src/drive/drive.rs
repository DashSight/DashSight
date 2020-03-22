/*
 * Copyright 2018 Alistair Francis <alistair@alistair23.me>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *    http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::display::*;
use crate::drive::prepare;
use crate::drive::read_track::Coord;
use gpsd_proto::{get_data, handshake, ResponseData};
use gtk;
use gtk::prelude::*;
use std::cell::Cell;
use std::cell::RefCell;
use std::io;
use std::net::TcpStream;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime};

struct LapTime {
    min: u64,
    sec: u64,
    msec: u32,
    nsec: u32,
}

struct Course {
    times: Vec<LapTime>,
    last: LapTime,
    best: LapTime,
    worst: LapTime,
    start: Coord,
    finish: Coord,
}

impl Course {
    fn new(start_lat: f64, start_lon: f64, finish_lat: f64, finish_lon: f64) -> Course {
        Course {
            times: Vec::new(),
            last: LapTime {
                min: 0,
                sec: 0,
                msec: 0,
                nsec: 0,
            },
            best: LapTime {
                min: 0,
                sec: 0,
                msec: 0,
                nsec: 0,
            },
            worst: LapTime {
                min: 0,
                sec: 0,
                msec: 0,
                nsec: 0,
            },
            start: Coord {
                lat: start_lat,
                lon: start_lon,
            },
            finish: Coord {
                lat: finish_lat,
                lon: finish_lon,
            },
        }
    }
}

struct Threading {
    lap_start: RefCell<std::time::SystemTime>,
    close: Mutex<Cell<bool>>,
    on_track: Mutex<Cell<bool>>,
    no_track: Mutex<Cell<bool>>,
    tx: std::sync::mpsc::Sender<(f64, f64)>,
}

type ThreadingRef = Arc<Threading>;

impl Threading {
    fn new(tx: std::sync::mpsc::Sender<(f64, f64)>) -> ThreadingRef {
        ThreadingRef::new(Self {
            lap_start: RefCell::new(SystemTime::now()),
            close: Mutex::new(Cell::new(false)),
            on_track: Mutex::new(Cell::new(false)),
            no_track: Mutex::new(Cell::new(false)),
            tx: tx,
        })
    }
}

unsafe impl Send for Threading {}
unsafe impl Sync for Threading {}

struct MapWrapper {
    path_layer: *mut champlain::path_layer::ChamplainPathLayer,
    point: *mut champlain::clutter::ClutterActor,
}

impl MapWrapper {
    fn new(
        path_layer: *mut champlain::path_layer::ChamplainPathLayer,
        champlain_point: *mut champlain::clutter::ClutterActor,
    ) -> MapWrapper {
        MapWrapper {
            path_layer: path_layer,
            point: champlain_point,
        }
    }
}

fn gpsd_thread(course_info: &mut Course, thread_info: ThreadingRef) {
    let gpsd_connect;

    loop {
        let stream = TcpStream::connect("127.0.0.1:2947");
        match stream {
            Ok(stream) => {
                gpsd_connect = stream;
                break;
            }
            Err(err) => {
                println!("Failed to connect to GPSD: {:?}", err);
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
        }
    }

    let mut reader = io::BufReader::new(&gpsd_connect);
    let mut writer = io::BufWriter::new(&gpsd_connect);

    let mut gpsd_message;

    handshake(&mut reader, &mut writer).unwrap();

    while !thread_info.close.lock().unwrap().get() {
        let msg = get_data(&mut reader);
        match msg {
            Ok(msg) => {
                gpsd_message = msg;
            }
            Err(err) => {
                println!("Failed to get a message from GPSD: {:?}", err);
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
        }

        match gpsd_message {
            ResponseData::Device(_) => {}
            ResponseData::Tpv(t) => {
                let lat = t.lat.unwrap_or(0.0);
                let lon = t.lon.unwrap_or(0.0);

                thread_info.tx.send((lat, lon)).unwrap();

                if !thread_info.on_track.lock().unwrap().get()
                    && lat == course_info.start.lat
                    && lon == course_info.start.lon
                {
                    thread_info.lap_start.replace(SystemTime::now());
                    thread_info.on_track.lock().unwrap().set(true);
                }

                if thread_info.on_track.lock().unwrap().get()
                    && lat == course_info.finish.lat
                    && lon == course_info.finish.lon
                {
                    thread_info.on_track.lock().unwrap().set(false);

                    match thread_info.lap_start.borrow().elapsed() {
                        Ok(elapsed) => {
                            course_info.times.push(LapTime {
                                min: elapsed.as_secs() / 60,
                                sec: elapsed.as_secs() % 60,
                                msec: elapsed.subsec_millis(),
                                nsec: elapsed.subsec_nanos(),
                            });
                        }
                        Err(e) => {
                            println!("Error: {:?}", e);
                        }
                    }
                }
            }
            ResponseData::Sky(_) => {}
            ResponseData::Pps(_) => {}
            ResponseData::Gst(_) => {}
        }
    }
}

fn map_update_idle_thread(
    rx: &std::sync::mpsc::Receiver<(f64, f64)>,
    map_wrapper: &MapWrapper,
    thread_info: ThreadingRef,
) -> glib::source::Continue {
    let timeout = Duration::new(0, 100);
    let rec = rx.recv_timeout(timeout);
    match rec {
        Ok((lat, lon)) => {
            champlain::location::set_location(
                champlain::clutter_actor::to_location(map_wrapper.point),
                lat,
                lon,
            );

            if thread_info.on_track.lock().unwrap().get() {
                let point_colour = champlain::clutter_colour::new(255, 200, 100, 255);
                champlain::point::set_colour(
                    champlain::clutter_actor::to_point(map_wrapper.point),
                    point_colour,
                );
            }

            if thread_info.no_track.lock().unwrap().get() {
                let coord = champlain::coordinate::new_full(lon, lat);
                champlain::path_layer::add_node(
                    map_wrapper.path_layer,
                    champlain::coordinate::to_location(coord),
                );
            }
            glib::source::Continue(true)
        }
        Err(mpsc::RecvTimeoutError::Timeout) => glib::source::Continue(true),
        _ => glib::source::Continue(false),
    }
}

fn time_update_idle_thread(
    builder: gtk::Builder,
    thread_info: ThreadingRef,
) -> glib::source::Continue {
    if thread_info.on_track.lock().unwrap().get() {
        match thread_info.lap_start.borrow().elapsed() {
            Ok(elapsed) => {
                let current_time = builder
                    .get_object::<gtk::Label>("CurrentTime")
                    .expect("Can't find CurrentTime in ui file.");

                let time = format!(
                    "{:02}:{:02}:{:02}",
                    elapsed.as_secs() / 60,
                    elapsed.as_secs() % 60,
                    elapsed.subsec_millis()
                );

                current_time.set_label(&time);
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }

    glib::source::Continue(true)
}

pub fn button_press_event(display: DisplayRef, track_sel_info: prepare::TrackSelectionRef) {
    let builder = display.builder.clone();

    let stack = builder
        .get_object::<gtk::Stack>("MainStack")
        .expect("Can't find MainStack in ui file.");
    stack.set_visible_child_name("DrivePage");

    let drive_page = builder
        .get_object::<gtk::Grid>("DriveGrid")
        .expect("Can't find DriveGrid in ui file.");

    let map_frame = builder
        .get_object::<gtk::Frame>("DriveMapFrame")
        .expect("Can't find DriveMapFrame in ui file.");
    map_frame.add(&track_sel_info.map_widget);

    let champlain_view = champlain::gtk_embed::get_view(track_sel_info.map_widget.clone())
        .expect("Unable to get ChamplainView");

    let track_points = track_sel_info.track_points.take();

    let (tx, rx) = mpsc::channel::<(f64, f64)>();
    let thread_info = Threading::new(tx);

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let _handler = thread::spawn(move || {
        let thread_info = upgrade_weak!(thread_info_weak);

        let mut course_info = Course::new(
            (&track_points).first().unwrap().lat,
            (&track_points).first().unwrap().lon,
            (&track_points).last().unwrap().lat,
            (&track_points).last().unwrap().lon,
        );

        gpsd_thread(&mut course_info, thread_info);
    });

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let display_weak = DisplayRef::downgrade(&display);
    gtk::idle_add(move || {
        let thread_info = upgrade_weak!(thread_info_weak, glib::source::Continue(false));
        let display = upgrade_weak!(display_weak, glib::source::Continue(false));

        let builder = display.builder.clone();

        time_update_idle_thread(builder, thread_info)
    });

    let layer = champlain::marker_layer::new();
    champlain::clutter_actor::show(champlain::layer::to_clutter_actor(
        champlain::marker_layer::to_layer(layer),
    ));
    champlain::view::add_layer(champlain_view, champlain::marker_layer::to_layer(layer));

    let point_colour = champlain::clutter_colour::new(100, 200, 255, 255);

    let point = champlain::point::new_full(12.0, point_colour);
    champlain::marker_layer::add_marker(
        layer,
        champlain::clutter_actor::to_champlain_marker(point),
    );

    let path_layer = champlain::path_layer::new();
    champlain::view::add_layer(champlain_view, champlain::path_layer::to_layer(path_layer));
    champlain::path_layer::set_visible(path_layer, true);

    champlain::marker_layer::show_all_markers(layer);

    let map_wrapper = MapWrapper::new(path_layer, point);

    let thread_info_clone = thread_info.clone();
    gtk::idle_add(move || {
        let thread_info = ThreadingRef::downgrade(&thread_info_clone)
            .upgrade()
            .unwrap();

        map_update_idle_thread(&rx, &map_wrapper, thread_info)
    });

    drive_page.show_all();
}
