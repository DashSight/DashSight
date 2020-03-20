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
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

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
    close: Mutex<Cell<bool>>,
    track_points: *mut Vec<crate::drive::read_track::Coord>,
    map_widget: gtk::Widget,
}

unsafe impl Send for Course {}
unsafe impl Sync for Course {}

type CourseRef = Arc<Course>;

impl Course {
    fn new(
        champlain_widget: gtk::Widget,
        track_points: *mut Vec<crate::drive::read_track::Coord>,
    ) -> CourseRef {
        CourseRef::new(Self {
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
            close: Mutex::new(Cell::new(false)),
            track_points: track_points,
            map_widget: champlain_widget,
        })
    }
}

fn run(course_info_weak: CourseRef) {
    let driving_lap: bool = false;
    let gpsd_connect;

    let course_info = course_info_weak.clone();
    let champlain_view = champlain::gtk_embed::get_view(course_info.map_widget.clone()).unwrap();

    loop {
        let stream = TcpStream::connect("127.0.0.1:2947");
        match stream {
            Ok(stream) => {
                gpsd_connect = stream;
                break;
            }
            Err(err) => {
                println!("Failed to connect to GPSD: {:?}", err);
                return;
            }
        }
    }

    let mut reader = io::BufReader::new(&gpsd_connect);
    let mut writer = io::BufWriter::new(&gpsd_connect);

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

    let mut gpsd_message;

    handshake(&mut reader, &mut writer).unwrap();

    while !course_info.close.lock().unwrap().get() {
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

                champlain::location::set_location(
                    champlain::clutter_actor::to_location(point),
                    lat,
                    lon,
                );

                champlain::marker_layer::show_all_markers(layer);

                if driving_lap {
                    let coord = champlain::coordinate::new_full(lon, lat);
                    champlain::path_layer::add_node(
                        path_layer,
                        champlain::coordinate::to_location(coord),
                    );
                }
            }
            ResponseData::Sky(_) => {}
            ResponseData::Pps(_) => {}
            ResponseData::Gst(_) => {}
        }
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

    let champlain_view = champlain::gtk_embed::get_view(track_sel_info.map_widget.clone())
        .expect("Unable to get ChamplainView");
    let champlain_actor = champlain::view::to_clutter_actor(champlain_view);

    champlain::view::set_kinetic_mode(champlain_view, true);
    champlain::view::set_zoom_on_double_click(champlain_view, true);
    champlain::view::set_zoom_level(champlain_view, 5);
    champlain::clutter_actor::set_reactive(champlain_actor, true);

    let map_frame = builder
        .get_object::<gtk::Frame>("DriveMapFrame")
        .expect("Can't find DriveMapFrame in ui file.");

    map_frame.add(&track_sel_info.map_widget);

    let course_info = Course::new(
        track_sel_info.map_widget.clone(),
        track_sel_info.track_points.as_ptr(),
    );

    let course_info_clone = course_info.clone();

    // let _handler = thread::spawn(move || {
    glib::source::idle_add(move || {
        let course_info = CourseRef::downgrade(&course_info_clone).upgrade().unwrap();

        run(course_info);
        glib::source::Continue(true)
    });

    drive_page.show_all();
}
