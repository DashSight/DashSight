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

use crate::drive::course::{Course, MapWrapper};
use crate::drive::obdii;
use crate::drive::obdii::OBDIICommandType;
use crate::utils::lat_lon_comp;
use gpsd_proto::handshake;
use gtk::prelude::*;
use plotters::prelude::*;
use std::cell::Cell;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

pub struct Threading {
    pub lap_start: RefCell<std::time::SystemTime>,
    pub close: Mutex<Cell<bool>>,
    pub on_track: Mutex<Cell<bool>>,
    pub change_colour: Mutex<Cell<bool>>,
    pub no_track: Mutex<Cell<bool>>,
    pub serialise: Mutex<Cell<bool>>,
    pub calibrate: Mutex<Cell<bool>>,
    pub time_file: RefCell<std::path::PathBuf>,
    pub location_tx: std::sync::mpsc::Sender<(f64, f64)>,
    pub times_tx: std::sync::mpsc::Sender<(Duration, Duration, Duration)>,
    pub obdii_tx: std::sync::mpsc::Sender<obdii::OBDIIData>,
    pub imu_tx: std::sync::mpsc::Sender<(f64, f64)>,
    pub imu_page_tx: std::sync::mpsc::Sender<(f64, f64)>,
    pub temp_tx: std::sync::mpsc::Sender<Vec<f64>>,
}

pub type ThreadingRef = Arc<Threading>;

unsafe impl Send for Threading {}
unsafe impl Sync for Threading {}

impl Threading {
    pub fn new(
        location_tx: std::sync::mpsc::Sender<(f64, f64)>,
        times_tx: std::sync::mpsc::Sender<(Duration, Duration, Duration)>,
        obdii_tx: std::sync::mpsc::Sender<obdii::OBDIIData>,
        imu_tx: std::sync::mpsc::Sender<(f64, f64)>,
        imu_page_tx: std::sync::mpsc::Sender<(f64, f64)>,
        temp_tx: std::sync::mpsc::Sender<Vec<f64>>,
    ) -> ThreadingRef {
        ThreadingRef::new(Self {
            lap_start: RefCell::new(SystemTime::now()),
            close: Mutex::new(Cell::new(false)),
            on_track: Mutex::new(Cell::new(false)),
            change_colour: Mutex::new(Cell::new(false)),
            no_track: Mutex::new(Cell::new(false)),
            serialise: Mutex::new(Cell::new(false)),
            calibrate: Mutex::new(Cell::new(false)),
            time_file: RefCell::new(PathBuf::new()),
            location_tx,
            times_tx,
            obdii_tx,
            imu_tx,
            imu_page_tx,
            temp_tx,
        })
    }

    pub fn gpsd_thread(&self, course_info: &mut Course) {
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
                    if self.close.lock().unwrap().get() {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    continue;
                }
            }
        }

        let mut reader = io::BufReader::new(&gpsd_connect);
        let mut writer = io::BufWriter::new(&gpsd_connect);

        handshake(&mut reader, &mut writer).unwrap();

        while !self.close.lock().unwrap().get() {
            let msg = crate::utils::get_gps_lat_lon(&mut reader);

            match msg {
                Ok((lat, lon, _alt, _time, _speed)) => {
                    self.location_tx.send((lat, lon)).unwrap();

                    if !self.on_track.lock().unwrap().get()
                        && lat_lon_comp(lat, lon, course_info.start.lat, course_info.start.lon)
                    {
                        self.lap_start.replace(SystemTime::now());
                        self.on_track.lock().unwrap().set(true);
                        self.change_colour.lock().unwrap().set(true);
                    }

                    if self.on_track.lock().unwrap().get()
                        && lat_lon_comp(lat, lon, course_info.finish.lat, course_info.finish.lon)
                    {
                        self.on_track.lock().unwrap().set(false);
                        self.change_colour.lock().unwrap().set(true);

                        match self.lap_start.borrow().elapsed() {
                            Ok(elapsed) => {
                                course_info.times.push(elapsed);
                                course_info.last = elapsed;
                                course_info.times.sort_unstable();
                                if let Some(worst) = course_info.times.last() {
                                    course_info.worst = *worst
                                }
                                if let Some(best) = course_info.times.first() {
                                    course_info.best = *best
                                }
                                self.times_tx
                                    .send((course_info.last, course_info.best, course_info.worst))
                                    .unwrap();
                            }
                            Err(e) => {
                                println!("Error: {:?}", e);
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

            if self.serialise.lock().unwrap().get() {
                let mut track_file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(self.time_file.borrow().clone());

                match track_file.as_mut() {
                    Ok(fd) => {
                        let serialized = serde_json::to_string(&course_info).unwrap();

                        serde_json::to_writer(fd, &serialized).unwrap();
                    }
                    Err(e) => {
                        println!("Unable to open file: {:?}", e);
                    }
                }
            }
        }
    }

    pub fn time_update_idle_thread(
        &self,
        times_rx: &std::sync::mpsc::Receiver<(Duration, Duration, Duration)>,
        builder: gtk::Builder,
    ) -> glib::source::Continue {
        if self.on_track.lock().unwrap().get() {
            match self.lap_start.borrow().elapsed() {
                Ok(elapsed) => {
                    let current_time = builder
                        .get_object::<gtk::Label>("CurrentTime")
                        .expect("Can't find CurrentTime in ui file.");

                    let time = format!(
                        "{:02}:{:02}:{:03}",
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

        let timeout = Duration::new(0, 100);
        let rec = times_rx.recv_timeout(timeout);
        match rec {
            Ok((last, best, worst)) => {
                let last_time = builder
                    .get_object::<gtk::Label>("LastTime")
                    .expect("Can't find LastTime in ui file.");
                let time = format!(
                    "{:02}:{:02}:{:02}",
                    last.as_secs() / 60,
                    last.as_secs() % 60,
                    last.subsec_millis()
                );
                last_time.set_label(&time);

                let best_time = builder
                    .get_object::<gtk::Label>("BestTime")
                    .expect("Can't find BestTime in ui file.");
                let time = format!(
                    "{:02}:{:02}:{:02}",
                    best.as_secs() / 60,
                    best.as_secs() % 60,
                    best.subsec_millis()
                );
                best_time.set_label(&time);

                let worst_time = builder
                    .get_object::<gtk::Label>("WorstTime")
                    .expect("Can't find WorstTime in ui file.");
                let time = format!(
                    "{:02}:{:02}:{:02}",
                    worst.as_secs() / 60,
                    worst.as_secs() % 60,
                    worst.subsec_millis()
                );
                worst_time.set_label(&time);

                glib::source::Continue(true)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => glib::source::Continue(true),
            _ => glib::source::Continue(false),
        }
    }

    pub fn obdii_update_idle_thread(
        &self,
        obdii_rx: &std::sync::mpsc::Receiver<obdii::OBDIIData>,
        builder: gtk::Builder,
    ) -> glib::source::Continue {
        let timeout = Duration::new(0, 100);
        let rec = obdii_rx.recv_timeout(timeout);
        match rec {
            Ok(data) => {
                if data.command == OBDIICommandType::Rpm {
                } else if data.command == OBDIICommandType::Throttle {
                    let pbar = builder
                        .get_object::<gtk::ProgressBar>("ThrottleBar")
                        .expect("Can't find ThrottleBar in ui file.");
                    unsafe {
                        pbar.set_fraction(data.val.float / 100.0);
                    }

                    let chart = builder
                        .get_object::<gtk::DrawingArea>("OBDIIChartOne")
                        .expect("Can't find OBDIIChartOne in ui file.");

                    println!("About to draw the RPM graph");

                    chart.connect_draw(move |me, cr| {
                        let width = me.get_allocated_width() as f64;
                        let height = me.get_allocated_width() as f64 * 0.7;

                        let root = CairoBackend::new(cr, (500, 500))
                            .unwrap()
                            .into_drawing_area();

                        let mut chart = ChartBuilder::on(&root)
                            .margin(10)
                            .caption("RPM", ("sans-serif", 30).into_font())
                            .x_label_area_size(width as u32)
                            .y_label_area_size(height as u32)
                            .build_ranged(0..100 as u32, 0f32..1f32)
                            .unwrap();

                        chart.configure_mesh().draw().unwrap();

                        Inhibit(true)
                    });
                } else if data.command == OBDIICommandType::EngineLoad {
                    let pbar = builder
                        .get_object::<gtk::ProgressBar>("LoadBar")
                        .expect("Can't find LoadBar in ui file.");
                    unsafe {
                        pbar.set_fraction(data.val.float / 100.0);
                    }
                } else if data.command == OBDIICommandType::TimingAdv {
                    let label = builder
                        .get_object::<gtk::Label>("TimingAdvValue")
                        .expect("Can't find TimingAdvValue in ui file.");
                    let text;
                    unsafe {
                        text = format!("{:3.2}", data.val.float);
                    }
                    label.set_text(&text);
                } else if data.command == OBDIICommandType::Maf {
                    let label = builder
                        .get_object::<gtk::Label>("MAFValue")
                        .expect("Can't find MAFValue in ui file.");
                    let text;
                    unsafe {
                        text = format!("{:3.2}", data.val.float);
                    }
                    label.set_text(&text);
                } else if data.command == OBDIICommandType::CoolantTemp {
                    let label = builder
                        .get_object::<gtk::Label>("CoolantTempValue")
                        .expect("Can't find CoolantTempValue in ui file.");
                    let text;
                    unsafe {
                        text = format!("{:3}", data.val.long);
                    }
                    label.set_text(&text);
                } else if data.command == OBDIICommandType::IntakeTemp {
                    let label = builder
                        .get_object::<gtk::Label>("IntakeTempValue")
                        .expect("Can't find IntakeTempValue in ui file.");
                    let text;
                    unsafe {
                        text = format!("{:3}", data.val.long);
                    }
                    label.set_text(&text);
                } else if data.command == OBDIICommandType::ShortFuelT1 {
                    let label = builder
                        .get_object::<gtk::Label>("ShortFuelB1Value")
                        .expect("Can't find ShortFuelB1Value in ui file.");
                    let text;
                    unsafe {
                        text = format!("{:3}", data.val.float);
                    }
                    label.set_text(&text);
                } else if data.command == OBDIICommandType::LongFuelT1 {
                    let label = builder
                        .get_object::<gtk::Label>("LongFuelB1Value")
                        .expect("Can't find LongFuelB1Value in ui file.");
                    let text;
                    unsafe {
                        text = format!("{:3}", data.val.float);
                    }
                    label.set_text(&text);
                } else if data.command == OBDIICommandType::FuelStatus {
                }
                glib::source::Continue(true)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => glib::source::Continue(true),
            _ => glib::source::Continue(false),
        }
    }

    pub fn map_update_idle_thread(
        &self,
        location_rx: &std::sync::mpsc::Receiver<(f64, f64)>,
        map_wrapper: &MapWrapper,
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

                if self.change_colour.lock().unwrap().get() {
                    if self.on_track.lock().unwrap().get() {
                        let point_colour = champlain::clutter_colour::new(255, 120, 0, 255);
                        champlain::point::set_colour(
                            champlain::clutter_actor::to_point(map_wrapper.point),
                            point_colour,
                        );
                    } else {
                        let point_colour = champlain::clutter_colour::new(100, 200, 255, 255);
                        champlain::point::set_colour(
                            champlain::clutter_actor::to_point(map_wrapper.point),
                            point_colour,
                        );
                    }
                    self.change_colour.lock().unwrap().set(false);
                }

                if self.no_track.lock().unwrap().get() {
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

    pub fn imu_draw_idle_thread(
        &self,
        imu_rx: &std::sync::mpsc::Receiver<(f64, f64)>,
        me: &gtk::DrawingArea,
        ctx: &cairo::Context,
    ) -> glib::signal::Inhibit {
        let timeout = Duration::new(0, 200);
        let rec = imu_rx.recv_timeout(timeout);

        let width = me.get_allocated_width() as f64;
        let height = me.get_allocated_width() as f64 * 0.7;

        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.9);
        ctx.set_line_width(0.2);

        // draw circles
        ctx.arc(
            0.5 * width,
            0.5 * height,
            0.1 * height,
            0.0,
            std::f64::consts::PI * 2.,
        );
        ctx.stroke();
        ctx.arc(
            0.5 * width,
            0.5 * height,
            0.2 * height,
            0.0,
            std::f64::consts::PI * 2.,
        );
        ctx.stroke();
        ctx.arc(
            0.5 * width,
            0.5 * height,
            0.3 * height,
            0.0,
            std::f64::consts::PI * 2.,
        );
        ctx.stroke();
        ctx.arc(
            0.5 * width,
            0.5 * height,
            0.4 * height,
            0.0,
            std::f64::consts::PI * 2.,
        );
        ctx.stroke();

        // draw border
        ctx.set_source_rgba(0.3, 0.3, 0.3, 1.0);
        ctx.rectangle(0.0, 0.0, 1.0 * width, 1.0 * height);
        ctx.stroke();

        ctx.set_line_width(0.5);

        // cross
        ctx.move_to(0.5 * width, 0.0);
        ctx.line_to(0.5 * width, height);
        ctx.stroke();
        ctx.move_to(0.0, 0.5 * height);
        ctx.line_to(width, 0.5 * height);
        ctx.stroke();

        match rec {
            Ok((x_accel, y_accel)) => {
                ctx.set_source_rgba(0.0, 148.0 / 255.0, 1.0, 1.0);

                ctx.arc(
                    (0.5 * width) + x_accel,
                    (0.5 * height) + y_accel,
                    5.0,
                    0.0,
                    std::f64::consts::PI * 2.,
                );
                ctx.fill();

                Inhibit(false)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Inhibit(false),
            _ => Inhibit(true),
        }
    }

    pub fn temp_update_idle_thread(
        &self,
        temp_rx: &std::sync::mpsc::Receiver<Vec<f64>>,
        builder: gtk::Builder,
    ) -> glib::source::Continue {
        let timeout = Duration::new(0, 100);
        let rec = temp_rx.recv_timeout(timeout);
        match rec {
            Ok(temps) => {
                for (i, temp) in temps.iter().enumerate() {
                    if i == 0 {
                        let label = builder
                            .get_object::<gtk::Label>("TopLeftTempLabel")
                            .expect("Can't find TopLeftTempLabel in ui file.");

                        let text = format!("{:2.2}", temp);
                        label.set_text(&text);
                    }
                }
                glib::source::Continue(true)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => glib::source::Continue(true),
            _ => glib::source::Continue(false),
        }
    }
}
