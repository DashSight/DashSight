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
use gtk::prelude::*;
use gtk::ResponseType;
use std::cell::Cell;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Error;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Duration;

pub struct MapWrapper {
    champlain_view: champlain::view::ChamplainView,
    path_layer: champlain::path_layer::ChamplainPathLayer,
    point: champlain::point::ChamplainPoint,
}

impl MapWrapper {
    pub fn new(
        champlain_view: champlain::view::ChamplainView,
        path_layer: champlain::path_layer::ChamplainPathLayer,
        champlain_point: champlain::point::ChamplainPoint,
    ) -> MapWrapper {
        MapWrapper {
            champlain_view,
            path_layer,
            point: champlain_point,
        }
    }
}

pub struct RecordInfo {
    track_file: RwLock<std::path::PathBuf>,
    new_file: Mutex<Cell<bool>>,
    save: Mutex<Cell<bool>>,
    toggle_save: Mutex<Cell<bool>>,
    pub close: Mutex<Cell<bool>>,
}

pub type RecordInfoRef = Arc<RecordInfo>;

impl RecordInfo {
    pub fn new() -> RecordInfoRef {
        RecordInfoRef::new(Self {
            track_file: RwLock::new(PathBuf::new()),
            new_file: Mutex::new(Cell::new(false)),
            save: Mutex::new(Cell::new(false)),
            toggle_save: Mutex::new(Cell::new(false)),
            close: Mutex::new(Cell::new(false)),
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
                let mut track_file = self.track_file.write().unwrap();
                *track_file = filepath;
            }
        }
    }

    pub fn record_button_clicked(&self) {
        let val = self.save.lock().unwrap().get();
        self.save.lock().unwrap().set(!val);

        if self.track_file.read().unwrap().exists() {
            self.toggle_save.lock().unwrap().set(true);
        }
    }

    pub fn idle_thread(
        &self,
        location_rx: &std::sync::mpsc::Receiver<(f64, f64, i32)>,
        map_wrapper: &mut MapWrapper,
        first_connect: &mut bool,
    ) -> glib::source::Continue {
        let timeout = Duration::new(0, 100);
        let rec = location_rx.recv_timeout(timeout);
        match rec {
            Ok((lat, lon, status)) => {
                crate::utils::set_point_colour(&mut map_wrapper.point, status);

                map_wrapper
                    .point
                    .borrow_mut_location()
                    .set_location(lat, lon);

                if *first_connect {
                    map_wrapper.champlain_view.set_zoom_level(17);
                    map_wrapper.champlain_view.center_on(lat, lon);
                    *first_connect = false;
                }

                if self.save.lock().unwrap().get() {
                    let mut coord = champlain::coordinate::ChamplainCoordinate::new_full(lat, lon);
                    map_wrapper.path_layer.add_node(coord.borrow_mut_location());
                }
                glib::source::Continue(true)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => glib::source::Continue(true),
            _ => glib::source::Continue(false),
        }
    }

    pub fn run(&self, location_tx: std::sync::mpsc::Sender<(f64, f64, i32)>) {
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
                    .open(self.track_file.read().unwrap().clone());

                if let Ok(mut fd) = track_file.as_mut() {
                    print::gpx_start(&mut fd).unwrap();
                    print::gpx_metadata(&mut fd).unwrap();
                    if let Some(filename) = self.track_file.read().unwrap().file_name() {
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
                Ok((lat, lon, alt, status, time, speed, track)) => {
                    if location_tx.send((lat, lon, status)).is_err() {
                        break;
                    }

                    if self.save.lock().unwrap().get() && !self.toggle_save.lock().unwrap().get() {
                        if let Ok(mut fd) = track_file.as_mut() {
                            // Only record the point if we are moving
                            if speed > 0.5 {
                                print::gpx_point_info(&mut fd, lat, lon, alt, time, track).unwrap();
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
        let (location_tx, _location_rx) = mpsc::channel::<(f64, f64, i32)>();
        let rec_info = RecordInfo::new();

        // Tell run to exit straight away, otherwise we loop for a GPSD conection
        rec_info.close.lock().unwrap().set(true);

        rec_info.run(location_tx);
    }
}
