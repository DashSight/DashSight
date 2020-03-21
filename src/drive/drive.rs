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
use gpsd_proto::{get_data, handshake, ResponseData};
use gtk;
use gtk::prelude::*;
use std::cell::Cell;
use std::io;
use std::net::TcpStream;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

struct LapTime {
    min: u64,
    sec: u64,
    nsec: u64,
}

struct Course {
    times: Vec<LapTime>,
    last: LapTime,
    best: LapTime,
    worst: LapTime,
    // track_points: *mut Vec<crate::drive::read_track::Coord>,
}

impl Course {
    fn new(_track_points: *mut Vec<crate::drive::read_track::Coord>) -> Course {
        Course {
            times: Vec::new(),
            last: LapTime {
                min: 0,
                sec: 0,
                nsec: 0,
            },
            best: LapTime {
                min: 0,
                sec: 0,
                nsec: 0,
            },
            worst: LapTime {
                min: 0,
                sec: 0,
                nsec: 0,
            },
            // track_points: track_points
        }
    }
}

struct Threading {
    close: Mutex<Cell<bool>>,
    on_track: Mutex<Cell<bool>>,
    tx: std::sync::mpsc::Sender<(f64, f64)>,
}

type ThreadingRef = Arc<Threading>;

impl Threading {
    fn new(tx: std::sync::mpsc::Sender<(f64, f64)>) -> ThreadingRef {
        ThreadingRef::new(Self {
            close: Mutex::new(Cell::new(false)),
            on_track: Mutex::new(Cell::new(false)),
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

fn gpsd_thread(_course_info: Course, thread_info_weak: ThreadingRef) {
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

    while !thread_info_weak.close.lock().unwrap().get() {
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

                thread_info_weak.tx.send((lat, lon)).unwrap();
            }
            ResponseData::Sky(_) => {}
            ResponseData::Pps(_) => {}
            ResponseData::Gst(_) => {}
        }
    }
}

fn idle_thread(
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

    let course_info = Course::new(track_sel_info.track_points.as_ptr());

    let (tx, rx) = mpsc::channel::<(f64, f64)>();
    let thread_info = Threading::new(tx);

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let _handler = thread::spawn(move || {
        let thread_info = upgrade_weak!(thread_info_weak);
        gpsd_thread(course_info, thread_info);
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

        idle_thread(&rx, &map_wrapper, thread_info)
    });

    drive_page.show_all();
}
