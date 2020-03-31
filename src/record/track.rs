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
use crate::record::print;
use chrono::DateTime;
use gpsd_proto::handshake;
use gtk;
use gtk::prelude::*;
use gtk::ResponseType;
use std::cell::Cell;
use std::cell::RefCell;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Error;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

struct RecordInfo {
    track_file: RefCell<std::path::PathBuf>,
    new_file: Mutex<Cell<bool>>,
    save: Mutex<Cell<bool>>,
    toggle_save: Mutex<Cell<bool>>,
    close: Mutex<Cell<bool>>,
    location_tx: std::sync::mpsc::Sender<(f64, f64)>,
}

unsafe impl Send for RecordInfo {}
unsafe impl Sync for RecordInfo {}

type RecordInfoRef = Arc<RecordInfo>;

impl RecordInfo {
    fn new(location_tx: std::sync::mpsc::Sender<(f64, f64)>) -> RecordInfoRef {
        RecordInfoRef::new(Self {
            track_file: RefCell::new(PathBuf::new()),
            new_file: Mutex::new(Cell::new(false)),
            save: Mutex::new(Cell::new(false)),
            toggle_save: Mutex::new(Cell::new(false)),
            close: Mutex::new(Cell::new(false)),
            location_tx: location_tx,
        })
    }
}

struct MapWrapper {
    champlain_view: *mut champlain::view::ChamplainView,
    path_layer: *mut champlain::path_layer::ChamplainPathLayer,
    point: *mut champlain::clutter::ClutterActor,
}

impl MapWrapper {
    fn new(
        champlain_view: *mut champlain::view::ChamplainView,
        path_layer: *mut champlain::path_layer::ChamplainPathLayer,
        champlain_point: *mut champlain::clutter::ClutterActor,
    ) -> MapWrapper {
        MapWrapper {
            champlain_view: champlain_view,
            path_layer: path_layer,
            point: champlain_point,
        }
    }
}

fn file_picker_clicked(display: DisplayRef, rec_info: RecordInfoRef) {
    let builder = display.builder.clone();

    let window: gtk::ApplicationWindow = builder
        .get_object("MainPage")
        .expect("Couldn't find MainPage in ui file.");

    let file_chooser = gtk::FileChooserNative::new(
        Some("Save track as"),
        Some(&window),
        gtk::FileChooserAction::Save,
        Some("Save"),
        Some("Close"),
    );

    let response = file_chooser.run();
    if ResponseType::from(response) == ResponseType::Accept {
        if let Some(filepath) = file_chooser.get_filename() {
            rec_info.new_file.lock().unwrap().set(true);
            rec_info.track_file.replace(filepath);
        }
    }
}

fn record_button_clicked(display: DisplayRef, rec_info: RecordInfoRef) {
    let builder = display.builder.clone();
    let record_button = builder
        .get_object::<gtk::ToggleButton>("RecordButton")
        .expect("Can't find RecordButton in ui file.");

    let val = rec_info.save.lock().unwrap().get();
    rec_info.save.lock().unwrap().set(!val);

    if rec_info.track_file.borrow().exists() {
        record_button.set_active(true);
        if rec_info.save.lock().unwrap().get() {
            record_button.set_label("gtk-media-stop");
        } else {
            record_button.set_label("gtk-media-record");
        }
        rec_info.toggle_save.lock().unwrap().set(true);
    } else {
        record_button.set_active(false);
    }
}

fn location_idle_thread(
    location_rx: &std::sync::mpsc::Receiver<(f64, f64)>,
    map_wrapper: &MapWrapper,
    first_connect: &mut bool,
    rec_info: RecordInfoRef,
) -> glib::source::Continue {
    let timeout = Duration::new(0, 100);
    let rec = location_rx.recv_timeout(timeout);
    match rec {
        Ok((lat, lon)) => {
            champlain::location::set_location(
                champlain::clutter_actor::to_location(map_wrapper.point),
                lat,
                lon,
            );

            if *first_connect {
                champlain::view::set_zoom_level(map_wrapper.champlain_view, 17);
                champlain::view::center_on(map_wrapper.champlain_view, lat, lon);
                *first_connect = false;
            }

            if rec_info.save.lock().unwrap().get() {
                let coord = champlain::coordinate::new_full(lat, lon);
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

fn run(rec_info_weak: RecordInfoRef) {
    let gpsd_connect;
    let mut average_lat_lon: Option<(f64, f64)> = None;

    let rec_info = rec_info_weak.clone();

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

    let mut track_file: Result<File, std::io::Error> =
        Err(Error::new(std::io::ErrorKind::NotFound, "No file yet"));

    handshake(&mut reader, &mut writer).unwrap();

    let mut kalman_filter = crate::utils::Kalman::new(15.0);

    while !rec_info.close.lock().unwrap().get() {
        if rec_info.new_file.lock().unwrap().get() {
            track_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(rec_info.track_file.borrow().clone());

            match track_file.as_mut() {
                Ok(mut fd) => {
                    print::gpx_start(&mut fd).unwrap();
                    print::gpx_metadata(&mut fd).unwrap();
                    if let Some(filename) = rec_info.track_file.borrow().file_name() {
                        if let Some(name) = filename.to_str() {
                            print::gpx_track_start(&mut fd, name.to_string()).unwrap();
                        }
                    }
                }
                _ => {}
            }
            rec_info.new_file.lock().unwrap().set(false);
        }

        if rec_info.toggle_save.lock().unwrap().get() {
            match track_file.as_mut() {
                Ok(mut fd) => {
                    if rec_info.save.lock().unwrap().get() {
                        print::gpx_track_seg_start(&mut fd).unwrap();
                    } else {
                        print::gpx_track_seg_stop(&mut fd).unwrap();
                    }
                }
                _ => {}
            }
            rec_info.toggle_save.lock().unwrap().set(false);
        }

        let msg = crate::utils::get_gps_lat_lon(&mut reader);

        match msg {
            Ok((unfilt_lat, unfilt_lon, errors, alt, time, speed)) => {
                let (lat, lon) = kalman_filter.process(
                    unfilt_lat,
                    unfilt_lon,
                    errors,
                    DateTime::parse_from_rfc3339(&time)
                        .unwrap()
                        .timestamp_millis(),
                    (speed + 2.0).round(),
                );

                if speed < 1.0 {
                    match average_lat_lon {
                        Some(mut avg) => {
                            avg.0 = (avg.0 + lat) / 2.0;
                            avg.1 = (avg.1 + lat) / 2.0;
                        }
                        None => {
                            average_lat_lon = Some((lat, lon));
                        }
                    }

                    rec_info
                        .location_tx
                        .send((average_lat_lon.unwrap().0, average_lat_lon.unwrap().1))
                        .unwrap();
                } else {
                    rec_info.location_tx.send((lat, lon)).unwrap();

                    average_lat_lon = None;

                    if rec_info.save.lock().unwrap().get()
                        && !rec_info.toggle_save.lock().unwrap().get()
                    {
                        match track_file.as_mut() {
                            Ok(mut fd) => {
                                print::gpx_point_info(&mut fd, lat, lon, alt, time).unwrap();
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(err) => {
                println!("Failed to get a message from GPSD: {:?}", err);
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
        }
    }

    match track_file.as_mut() {
        Ok(mut fd) => {
            print::gpx_track_stop(&mut fd).unwrap();
            print::gpx_stop(&mut fd).unwrap();
            fd.sync_all().unwrap();
        }
        _ => {}
    }
}

pub fn button_press_event(display: DisplayRef) {
    let builder = display.builder.clone();

    let stack = builder
        .get_object::<gtk::Stack>("MainStack")
        .expect("Can't find MainStack in ui file.");

    stack.set_visible_child_name("RecordPage");

    let record_page = builder
        .get_object::<gtk::Paned>("RecordPage")
        .expect("Can't find RecordPage in ui file.");

    let clutter_init_error = champlain::gtk_clutter::init();
    if clutter_init_error != champlain::gtk_clutter::Error::CLUTTER_INIT_SUCCESS {
        println!("Unable to init clutter");
        process::exit(0);
    }

    let champlain_widget = champlain::gtk_embed::new();
    let champlain_view = champlain::gtk_embed::get_view(champlain_widget.clone())
        .expect("Unable to get ChamplainView");
    let champlain_actor = champlain::view::to_clutter_actor(champlain_view);

    champlain::view::set_kinetic_mode(champlain_view, true);
    champlain::view::set_zoom_on_double_click(champlain_view, true);
    champlain::view::set_zoom_level(champlain_view, 5);
    champlain::clutter_actor::set_reactive(champlain_actor, true);

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

    let map_frame = builder
        .get_object::<gtk::Frame>("RecordPageMapFrame")
        .expect("Can't find RecordPageMapFrame in ui file.");

    map_frame.add(&champlain_widget);

    record_page.pack1(&map_frame, true, true);

    let (location_tx, location_rx) = mpsc::channel::<(f64, f64)>();
    let rec_info = RecordInfo::new(location_tx);
    let map_wrapper = MapWrapper::new(champlain_view, path_layer, point);
    let mut first_connect = true;

    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    gtk::timeout_add(10, move || {
        let rec_info = upgrade_weak!(rec_info_weak, glib::source::Continue(false));

        if rec_info.close.lock().unwrap().get() {
            return glib::source::Continue(false);
        }

        location_idle_thread(&location_rx, &map_wrapper, &mut first_connect, rec_info)
    });

    let file_picker_button = builder
        .get_object::<gtk::Button>("RecordFileSaveButton")
        .expect("Can't find RecordFileSaveButton in ui file.");

    let display_weak = DisplayRef::downgrade(&display);
    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    file_picker_button.connect_clicked(move |_| {
        let display = upgrade_weak!(display_weak);
        let rec_info = upgrade_weak!(rec_info_weak);
        file_picker_clicked(display, rec_info);
    });

    let record_button = builder
        .get_object::<gtk::ToggleButton>("RecordButton")
        .expect("Can't find RecordButton in ui file.");

    record_button.set_active(false);

    let display_weak = DisplayRef::downgrade(&display);
    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    record_button.connect_clicked(move |_| {
        let display = upgrade_weak!(display_weak);
        let rec_info = upgrade_weak!(rec_info_weak);
        record_button_clicked(display, rec_info);
    });

    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    let _handler = thread::spawn(move || {
        let rec_info = rec_info_weak.upgrade().unwrap();
        run(rec_info)
    });

    let back_button = builder
        .get_object::<gtk::Button>("RecordBackButton")
        .expect("Can't find RecordBackButton in ui file.");

    // We use a strong reference here to make sure that rec_info isn't dropped
    let rec_info_clone = rec_info.clone();
    back_button.connect_clicked(move |_| {
        let rec_info = RecordInfoRef::downgrade(&rec_info_clone).upgrade().unwrap();
        rec_info.close.lock().unwrap().set(true);

        // handler.join().unwrap();

        stack.set_visible_child_name("SplashImage");
    });

    record_page.show_all();
}
