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
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

pub struct MapWrapper {
    champlain_view: *mut champlain::view::ChamplainView,
    path_layer: *mut champlain::path_layer::ChamplainPathLayer,
    point: *mut champlain::clutter::ClutterActor,
}

impl MapWrapper {
    pub fn new(
        champlain_view: *mut champlain::view::ChamplainView,
        path_layer: *mut champlain::path_layer::ChamplainPathLayer,
        champlain_point: *mut champlain::clutter::ClutterActor,
    ) -> MapWrapper {
        MapWrapper {
            champlain_view,
            path_layer,
            point: champlain_point,
        }
    }
}

pub struct RecordInfo {
    track_file: RefCell<std::path::PathBuf>,
    new_file: Mutex<Cell<bool>>,
    save: Mutex<Cell<bool>>,
    toggle_save: Mutex<Cell<bool>>,
    pub close: Mutex<Cell<bool>>,
    location_tx: std::sync::mpsc::Sender<(f64, f64)>,
}

unsafe impl Send for RecordInfo {}
unsafe impl Sync for RecordInfo {}

pub type RecordInfoRef = Arc<RecordInfo>;

impl RecordInfo {
    pub fn new(location_tx: std::sync::mpsc::Sender<(f64, f64)>) -> RecordInfoRef {
        RecordInfoRef::new(Self {
            track_file: RefCell::new(PathBuf::new()),
            new_file: Mutex::new(Cell::new(false)),
            save: Mutex::new(Cell::new(false)),
            toggle_save: Mutex::new(Cell::new(false)),
            close: Mutex::new(Cell::new(false)),
            location_tx,
        })
    }

    pub fn file_picker_clicked(&self, display: DisplayRef) {
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
        if response == ResponseType::Accept {
            if let Some(filepath) = file_chooser.get_filename() {
                self.new_file.lock().unwrap().set(true);
                self.track_file.replace(filepath);
            }
        }
    }

    pub fn record_button_clicked(&self) {
        let val = self.save.lock().unwrap().get();
        self.save.lock().unwrap().set(!val);

        if val && self.track_file.borrow().exists() {
            self.toggle_save.lock().unwrap().set(true);
        }
    }

    pub fn idle_thread(
        &self,
        location_rx: &std::sync::mpsc::Receiver<(f64, f64)>,
        map_wrapper: &MapWrapper,
        first_connect: &mut bool,
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

                if self.save.lock().unwrap().get() {
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

    pub fn run(&self) {
        let gpsd_connect;

        loop {
            let stream = TcpStream::connect("127.0.0.1:2947");
            match stream {
                Ok(stream) => {
                    gpsd_connect = stream;
                    break;
                }
                Err(err) => {
                    if self.close.lock().unwrap().get() {
                        return;
                    }

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

        while !self.close.lock().unwrap().get() {
            if self.new_file.lock().unwrap().get() {
                track_file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(self.track_file.borrow().clone());

                if let Ok(mut fd) = track_file.as_mut() {
                    print::gpx_start(&mut fd).unwrap();
                    print::gpx_metadata(&mut fd).unwrap();
                    if let Some(filename) = self.track_file.borrow().file_name() {
                        if let Some(name) = filename.to_str() {
                            print::gpx_track_start(&mut fd, name.to_string()).unwrap();
                        }
                    }
                }
                self.new_file.lock().unwrap().set(false);
            }

            if self.toggle_save.lock().unwrap().get() {
                if let Ok(mut fd) = track_file.as_mut() {
                    if self.save.lock().unwrap().get() {
                        print::gpx_track_seg_start(&mut fd).unwrap();
                    } else {
                        print::gpx_track_seg_stop(&mut fd).unwrap();
                    }
                }
                self.toggle_save.lock().unwrap().set(false);
            }

            let msg = crate::utils::get_gps_lat_lon(&mut reader);

            match msg {
                Ok((lat, lon, alt, time, _speed)) => {
                    self.location_tx.send((lat, lon)).unwrap();

                    if self.save.lock().unwrap().get() && !self.toggle_save.lock().unwrap().get() {
                        if let Ok(mut fd) = track_file.as_mut() {
                            print::gpx_point_info(&mut fd, lat, lon, alt, time).unwrap();
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

        if let Ok(mut fd) = track_file.as_mut() {
            print::gpx_track_stop(&mut fd).unwrap();
            print::gpx_stop(&mut fd).unwrap();
            fd.sync_all().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_run() {
        let (location_tx, _location_rx) = mpsc::channel::<(f64, f64)>();
        let rec_info = RecordInfo::new(location_tx);

        // Tell run to exit straight away, otherwise we loop for a GPSD conection
        rec_info.close.lock().unwrap().set(true);

        rec_info.run();
    }
}
