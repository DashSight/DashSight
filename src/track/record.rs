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
use glib;
use gpsd_proto::{get_data, ResponseData};
use gtk;
use gtk::prelude::*;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;
use std::io::Error;
use std::net::TcpStream;
use std::process;
use std::sync::Arc;

pub struct RecordInfo {
    pub track_file: Result<File, std::io::Error>,
    pub save: bool,
}

pub type RecordInfoRef = Arc<RecordInfo>;

impl RecordInfo {
    pub fn new() -> RecordInfoRef {
        let default_error = Error::new(std::io::ErrorKind::NotFound, "No file yet");
        RecordInfoRef::new(Self {
            track_file: Err(default_error),
            save: false,
        })
    }
}

fn print_gpx_start(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"<?xml version=\"1.0\" encoding=\"utf-8\"?>\n")?;
    fd.write_all(b"<gpx version=\"1.1\" creator=\"DashSight\"\n")?;
    fd.write_all(b"        xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n")?;
    fd.write_all(b"        xmlns=\"http://www.topografix.com/GPX/1.1\"\n")?;
    fd.write_all(b"        xsi:schemaLocation=\"http://www.topografix.com/GPS/1/1\n")?;
    fd.write_all(b"        http://www.topografix.com/GPX/1/1/gpx.xsd\">\n")?;
    Ok(())
}

fn print_gpx_stop(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"</gpx>\n")?;
    Ok(())
}

fn print_gpx_metadata(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"  <metadata>\n")?;
    fd.write_all(b"    <link href=\"https://github.com/alistair23/DashSight\">\n")?;
    fd.write_all(b"      <text>DashSight</text>\n")?;
    fd.write_all(b"    </link>\n")?;
    fd.write_all(b"  </metadata>\n")?;
    Ok(())
}

fn print_gpx_track_start(fd: &mut File, track_name: String) -> Result<(), std::io::Error> {
    fd.write_all(b"  <trk>\n")?;
    write!(fd, "    <name>{}</name>\n", track_name)?;
    Ok(())
}

fn print_gpx_track_stop(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"    </trkseg>\n")?;
    fd.write_all(b"  </trk>\n")?;
    Ok(())
}

fn print_gpx_point_info(
    fd: &mut File,
    lat: f64,
    lon: f64,
    alt: f32,
    time: String,
) -> Result<(), std::io::Error> {
    write!(fd, "      <trkpt lat=\"{}\" lon=\"{}\">\n", lat, lon)?;
    write!(fd, "        <ele>{}git f</ele>\n", alt)?;
    write!(fd, "        <time>{}</time>\n", time)?;
    write!(fd, "      </trkpt>\n")?;
    Ok(())
}

fn print_gpx_track_seg_start(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"    <trkseg>\n")?;
    Ok(())
}

fn print_gpx_track_seg_stop(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"    </trkseg>\n")?;
    Ok(())
}

fn record_page_file_picker(display: DisplayRef, rec_info_weak: &mut RecordInfoRef) {
    let rec_info = std::sync::Arc::get_mut(rec_info_weak).unwrap();

    let builder = display.builder.clone();

    let file_picker_button = builder
        .get_object::<gtk::FileChooserButton>("RecordFileSaveButton")
        .expect("Can't find RecordFileSaveButton in ui file.");

    if let Some(filepath) = file_picker_button.get_filename() {
        let track_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filepath.clone());

        if let Ok(mut fd) = track_file {
            print_gpx_start(&mut fd).unwrap();
            print_gpx_metadata(&mut fd).unwrap();
            if let Some(filename) = filepath.file_name() {
                if let Some(name) = filename.to_str() {
                    print_gpx_track_start(&mut fd, name.to_string()).unwrap();
                }
            }

            rec_info.track_file = fd.try_clone();
        }
    }
}

fn record_page_record_button(display: DisplayRef, rec_info_weak: &mut RecordInfoRef) {
    let rec_info = std::sync::Arc::get_mut(rec_info_weak).unwrap();

    let builder = display.builder.clone();
    let record_button = builder
        .get_object::<gtk::ToggleButton>("RecordButton")
        .expect("Can't find RecordButton in ui file.");

    rec_info.save = !rec_info.save;

    if rec_info.track_file.is_ok() {
        let mut track = rec_info.track_file.as_mut().unwrap();
        if rec_info.save {
            record_button.set_label("gtk-media-stop");
            print_gpx_track_seg_start(&mut track).unwrap();
        } else {
            record_button.set_label("gtk-media-record");
            print_gpx_track_seg_stop(&mut track).unwrap();
        }
    } else {
        record_button.set_active(false);
    }
}

fn record_page_run(rec_info_weak: RecordInfoRef) -> glib::source::Continue {
    let gpsd_connect;

    let stream = TcpStream::connect("127.0.0.1:2947");

    match stream {
        Ok(stream) => {
            gpsd_connect = stream;
        }
        Err(err) => {
            println!("Failed to connect to GPSD: {:?}", err);
            return glib::source::Continue(true);
        }
    }

    let mut reader = io::BufReader::new(&gpsd_connect);

    let marker = champlain::marker::new();

    let rec_info = rec_info_weak.clone();

    loop {
        let gpsd_message;
        let msg = get_data(&mut reader);
        match msg {
            Ok(msg) => {
                gpsd_message = msg;
            }
            Err(err) => {
                println!("Failed to get a message from GPSD: {:?}", err);
                continue;
            }
        }

        match gpsd_message {
            ResponseData::Device(_) => {}
            ResponseData::Version(_) => {}
            ResponseData::Tpv(t) => {
                println!(
                    "{:3} {:8.5} {:8.5} {:6.1} m {:5.1} ° {:6.3} m/s",
                    t.mode.to_string(),
                    t.lat.unwrap_or(0.0),
                    t.lon.unwrap_or(0.0),
                    t.alt.unwrap_or(0.0),
                    t.track.unwrap_or(0.0),
                    t.speed.unwrap_or(0.0),
                );
                champlain::location::set_location(
                    champlain::location::actor_to_location(marker),
                    t.lat.unwrap_or(0.0),
                    t.lon.unwrap_or(0.0),
                );

                if rec_info.track_file.is_ok() {
                    if let Ok(mut track) = rec_info.track_file.as_ref().unwrap().try_clone() {
                        print_gpx_point_info(
                            &mut track,
                            t.lat.unwrap_or(0.0),
                            t.lon.unwrap_or(0.0),
                            t.alt.unwrap_or(0.0),
                            t.time.unwrap_or("".to_string()),
                        )
                        .unwrap();
                    }
                }
            }
            ResponseData::Sky(_) => {}
            ResponseData::Pps(_) => {}
            ResponseData::Gst(_) => {}
        }
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
    let _champlain_view = champlain::gtk_embed::get_view(champlain_widget.clone())
        .expect("Unable to get ChamplainView");

    let map_frame = builder
        .get_object::<gtk::Frame>("RecordPageMapFrame")
        .expect("Can't find RecordPageMapFrame in ui file.");

    map_frame.add(&champlain_widget);

    record_page.pack1(&map_frame, true, true);

    let rec_info = RecordInfo::new();

    let file_picker_button = builder
        .get_object::<gtk::FileChooserButton>("RecordFileSaveButton")
        .expect("Can't find RecordFileSaveButton in ui file.");

    let display_weak = DisplayRef::downgrade(&display);
    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    file_picker_button.connect_file_set(move |_| {
        let display = upgrade_weak!(display_weak);
        let mut rec_info = upgrade_weak!(rec_info_weak);
        record_page_file_picker(display, &mut rec_info);
    });

    let record_button = builder
        .get_object::<gtk::ToggleButton>("RecordButton")
        .expect("Can't find RecordButton in ui file.");

    let display_weak = DisplayRef::downgrade(&display);
    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    record_button.connect_clicked(move |_| {
        let display = upgrade_weak!(display_weak);
        let mut rec_info = upgrade_weak!(rec_info_weak);
        record_page_record_button(display, &mut rec_info);
    });

    let back_button = builder
        .get_object::<gtk::Button>("RecordBackButton")
        .expect("Can't find RecordBackButton in ui file.");

    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    // We use a strong reference here to make sure that rec_info isn't dropped
    let rec_info_clone = rec_info.clone();
    back_button.connect_clicked(move |_| {
        let _rec_info_weak = RecordInfoRef::downgrade(&rec_info_clone).upgrade().unwrap();

        let mut rec_info = upgrade_weak!(rec_info_weak);
        let rec_info_mut = std::sync::Arc::get_mut(&mut rec_info);

        if let Some(ri) = rec_info_mut {
            if ri.track_file.is_ok() {
                let mut track = ri.track_file.as_mut().unwrap();
                print_gpx_track_stop(&mut track).unwrap();
                print_gpx_stop(&mut track).unwrap();
                track.sync_all().unwrap();
            }
        }
        stack.set_visible_child_name("SplashImage");
    });

    record_page.show_all();

    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    glib::source::idle_add(move || {
        let rec_info = rec_info_weak.upgrade().unwrap();
        record_page_run(rec_info)
    });
}
