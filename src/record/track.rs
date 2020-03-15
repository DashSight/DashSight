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
use gpsd_proto::{get_data, handshake, ResponseData};
use gtk;
use gtk::prelude::*;
use std::cell::Cell;
use std::cell::RefCell;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Error;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

struct RecordInfo {
    track_file: RefCell<std::path::PathBuf>,
    new_file: Mutex<Cell<bool>>,
    save: Mutex<Cell<bool>>,
    toggle_save: Mutex<Cell<bool>>,
    close: Mutex<Cell<bool>>,
    map: NonNull<champlain::view::ChamplainView>,
}

unsafe impl Send for RecordInfo {}
unsafe impl Sync for RecordInfo {}

type RecordInfoRef = Arc<RecordInfo>;

impl RecordInfo {
    fn new(champlain_view: *mut champlain::view::ChamplainView) -> RecordInfoRef {
        RecordInfoRef::new(Self {
            track_file: RefCell::new(PathBuf::new()),
            new_file: Mutex::new(Cell::new(false)),
            save: Mutex::new(Cell::new(false)),
            toggle_save: Mutex::new(Cell::new(false)),
            close: Mutex::new(Cell::new(false)),
            map: NonNull::new(champlain_view).unwrap(),
        })
    }
}

fn file_picker_clicked(display: DisplayRef, rec_info: RecordInfoRef) {
    let builder = display.builder.clone();

    let file_picker_button = builder
        .get_object::<gtk::FileChooserButton>("RecordFileSaveButton")
        .expect("Can't find RecordFileSaveButton in ui file.");

    if let Some(filepath) = file_picker_button.get_filename() {
        rec_info.new_file.lock().unwrap().set(true);
        rec_info.track_file.replace(filepath);
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
        if rec_info.track_file.borrow().exists() {
            if rec_info.save.lock().unwrap().get() {
                record_button.set_label("gtk-media-stop");
            } else {
                record_button.set_label("gtk-media-record");
            }
            rec_info.toggle_save.lock().unwrap().set(true);
        }
    } else {
        record_button.set_active(false);
    }
}

fn run(rec_info_weak: RecordInfoRef) {
    let gpsd_connect;
    let mut first_connect = true;

    let rec_info = rec_info_weak.clone();
    let champlain_view = rec_info.map.as_ptr();

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

    let layer = champlain::markerlayer::new();
    champlain::clutter_actor::show(champlain::layer::to_clutter_actor(
        champlain::markerlayer::to_layer(layer),
    ));
    champlain::view::add_layer(champlain_view, champlain::markerlayer::to_layer(layer));

    let point_colour = champlain::clutter_colour::new(100, 200, 255, 255);

    let point = champlain::point::new_full(12.0, point_colour);
    champlain::markerlayer::add_marker(layer, champlain::clutter_actor::to_champlain_marker(point));

    let path_layer = champlain::path_layer::new();
    champlain::view::add_layer(champlain_view, champlain::path_layer::to_layer(path_layer));
    champlain::path_layer::set_visible(path_layer, true);

    let mut gpsd_message;
    let mut track_file: Result<File, std::io::Error> =
        Err(Error::new(std::io::ErrorKind::NotFound, "No file yet"));

    handshake(&mut reader, &mut writer).unwrap();

    while !rec_info.close.lock().unwrap().get() {
        if rec_info.new_file.lock().unwrap().get() {
            track_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
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

                if first_connect {
                    champlain::markerlayer::animate_in_all_markers(layer);
                    champlain::view::set_zoom_level(champlain_view, 15);
                    champlain::view::center_on(champlain_view, lat, lon);
                    first_connect = false;
                }

                champlain::markerlayer::show_all_markers(layer);

                if rec_info.save.lock().unwrap().get() {
                    let coord = champlain::coordinate::new_full(lon, lat);
                    champlain::path_layer::add_node(
                        path_layer,
                        champlain::coordinate::to_location(coord),
                    );

                    match track_file.as_mut() {
                        Ok(mut fd) => {
                            print::gpx_point_info(
                                &mut fd,
                                lat,
                                lon,
                                t.alt.unwrap_or(0.0),
                                t.time.unwrap_or("".to_string()),
                            )
                            .unwrap();
                        }
                        _ => {}
                    }
                }
            }
            ResponseData::Sky(_) => {}
            ResponseData::Pps(_) => {}
            ResponseData::Gst(_) => {}
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

    let map_frame = builder
        .get_object::<gtk::Frame>("RecordPageMapFrame")
        .expect("Can't find RecordPageMapFrame in ui file.");

    map_frame.add(&champlain_widget);

    record_page.pack1(&map_frame, true, true);

    let rec_info = RecordInfo::new(champlain_view);

    let file_picker_button = builder
        .get_object::<gtk::FileChooserButton>("RecordFileSaveButton")
        .expect("Can't find RecordFileSaveButton in ui file.");

    let display_weak = DisplayRef::downgrade(&display);
    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    file_picker_button.connect_file_set(move |_| {
        let display = upgrade_weak!(display_weak);
        let rec_info = upgrade_weak!(rec_info_weak);
        file_picker_clicked(display, rec_info);
    });

    let record_button = builder
        .get_object::<gtk::ToggleButton>("RecordButton")
        .expect("Can't find RecordButton in ui file.");

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