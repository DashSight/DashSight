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

use crate::drive::course::MapWrapper;
use crate::drive::obdii;
use crate::drive::obdii::OBDIICommandType;
use gtk::prelude::*;
use std::cell::Cell;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::{Duration, SystemTime};

pub struct Threading {
    pub(crate) lap_start: RwLock<std::time::SystemTime>,
    pub(crate) close: Mutex<Cell<bool>>,
    pub(crate) start_on_track: Mutex<Cell<bool>>,
    pub(crate) on_track: Mutex<Cell<bool>>,
    pub(crate) serialise: Mutex<Cell<bool>>,
    pub(crate) calibrate: Mutex<Cell<bool>>,
    pub(crate) time_file: RwLock<std::path::PathBuf>,
}

pub type ThreadingRef = Arc<Threading>;

impl Threading {
    pub fn new() -> ThreadingRef {
        ThreadingRef::new(Self {
            lap_start: RwLock::new(SystemTime::now()),
            close: Mutex::new(Cell::new(false)),
            start_on_track: Mutex::new(Cell::new(false)),
            on_track: Mutex::new(Cell::new(false)),
            serialise: Mutex::new(Cell::new(false)),
            calibrate: Mutex::new(Cell::new(false)),
            time_file: RwLock::new(PathBuf::new()),
        })
    }

    pub fn time_update_idle_thread(
        &self,
        times_rx: &std::sync::mpsc::Receiver<(Duration, Duration, Duration)>,
        time_diff_rx: &std::sync::mpsc::Receiver<(bool, Duration)>,
        builder: gtk::Builder,
    ) -> glib::source::Continue {
        let timeout = Duration::new(0, 100);

        if self.on_track.lock().unwrap().get() {
            match self.lap_start.read().unwrap().elapsed() {
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

            let rec = time_diff_rx.recv_timeout(timeout);
            match rec {
                Ok((neg, diff)) => {
                    let time_diff = builder
                        .get_object::<gtk::Label>("BestDiff")
                        .expect("Can't find BestDiff in ui file.");
                    let sign = match neg {
                        true => "-",
                        false => "+",
                    };
                    let time = format!(
                        "{}{:02}:{:02}:{:03}",
                        sign,
                        diff.as_secs() / 60,
                        diff.as_secs() % 60,
                        diff.subsec_millis()
                    );
                    time_diff.set_label(&time);

                    if neg {
                        let negbar = builder
                            .get_object::<gtk::ProgressBar>("NegativeDiff")
                            .expect("Can't find NegativeDiff in ui file.");
                        negbar.set_fraction(diff.as_secs_f64() / 10.0);
                        let posbar = builder
                            .get_object::<gtk::ProgressBar>("PositiveDiff")
                            .expect("Can't find PositiveDiff in ui file.");
                        posbar.set_fraction(0.0);
                    } else {
                        let negbar = builder
                            .get_object::<gtk::ProgressBar>("NegativeDiff")
                            .expect("Can't find NegativeDiff in ui file.");
                        negbar.set_fraction(0.0);
                        let posbar = builder
                            .get_object::<gtk::ProgressBar>("PositiveDiff")
                            .expect("Can't find PositiveDiff in ui file.");
                        posbar.set_fraction(diff.as_secs_f64() / 10.0);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                _ => {
                    return glib::source::Continue(false);
                }
            }
        }

        let rec = times_rx.recv_timeout(timeout);
        match rec {
            Ok((last, best, worst)) => {
                let last_time = builder
                    .get_object::<gtk::Label>("LastTime")
                    .expect("Can't find LastTime in ui file.");
                let time = format!(
                    "{:02}:{:02}:{:03}",
                    last.as_secs() / 60,
                    last.as_secs() % 60,
                    last.subsec_millis()
                );
                last_time.set_label(&time);

                let best_time = builder
                    .get_object::<gtk::Label>("BestTime")
                    .expect("Can't find BestTime in ui file.");
                let time = format!(
                    "{:02}:{:02}:{:03}",
                    best.as_secs() / 60,
                    best.as_secs() % 60,
                    best.subsec_millis()
                );
                best_time.set_label(&time);

                let worst_time = builder
                    .get_object::<gtk::Label>("WorstTime")
                    .expect("Can't find WorstTime in ui file.");
                let time = format!(
                    "{:02}:{:02}:{:03}",
                    worst.as_secs() / 60,
                    worst.as_secs() % 60,
                    worst.subsec_millis()
                );
                worst_time.set_label(&time);

                // If we aren't currently driving the main time is
                // the same as the last time. This avoids showing a small
                // delay between the recorded end time and when this
                // was last updated
                if !self.on_track.lock().unwrap().get() {
                    let current_time = builder
                        .get_object::<gtk::Label>("CurrentTime")
                        .expect("Can't find CurrentTime in ui file.");
                    let time = format!(
                        "{:02}:{:02}:{:03}",
                        last.as_secs() / 60,
                        last.as_secs() % 60,
                        last.subsec_millis()
                    );
                    current_time.set_label(&time);
                }

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
        obdii_data: &Rc<RefCell<obdii::OBDIIGraphData>>,
    ) -> glib::source::Continue {
        let timeout = Duration::new(0, 100);
        for _i in 0..20 {
            let rec = obdii_rx.recv_timeout(timeout);
            match rec {
                Ok(data) => {
                    if data.command == OBDIICommandType::Rpm {
                        obdii_data
                            .borrow_mut()
                            .rpm
                            .push_front(data.val_float.unwrap());
                        if obdii_data.borrow().rpm.len() > obdii::VECTOR_LEN {
                            obdii_data.borrow_mut().rpm.pop_back();
                        }

                        let chart = builder
                            .get_object::<gtk::DrawingArea>("OBDIIChartOne")
                            .expect("Can't find OBDIIChartOne in ui file.");

                        chart.queue_draw();
                    } else if data.command == OBDIICommandType::Throttle {
                        let pbar = builder
                            .get_object::<gtk::ProgressBar>("ThrottleBar")
                            .expect("Can't find ThrottleBar in ui file.");
                        pbar.set_fraction(data.val_float.unwrap() / 100.0);

                        obdii_data
                            .borrow_mut()
                            .throttle
                            .push_front(data.val_float.unwrap());
                        if obdii_data.borrow().throttle.len() > obdii::VECTOR_LEN {
                            obdii_data.borrow_mut().throttle.pop_back();
                        }

                        let chart = builder
                            .get_object::<gtk::DrawingArea>("OBDIIChartThree")
                            .expect("Can't find OBDIIChartThree in ui file.");

                        chart.queue_draw();
                    } else if data.command == OBDIICommandType::EngineLoad {
                        let pbar = builder
                            .get_object::<gtk::ProgressBar>("LoadBar")
                            .expect("Can't find LoadBar in ui file.");
                        pbar.set_fraction(data.val_float.unwrap() / 100.0);

                        obdii_data
                            .borrow_mut()
                            .load
                            .push_front(data.val_float.unwrap());
                        if obdii_data.borrow().load.len() > obdii::VECTOR_LEN {
                            obdii_data.borrow_mut().load.pop_back();
                        }

                        let chart = builder
                            .get_object::<gtk::DrawingArea>("OBDIIChartFour")
                            .expect("Can't find OBDIIChartFour in ui file.");

                        chart.queue_draw();
                    } else if data.command == OBDIICommandType::TimingAdv {
                        let label = builder
                            .get_object::<gtk::Label>("TimingAdvValue")
                            .expect("Can't find TimingAdvValue in ui file.");
                        let text;
                        text = format!("{:3.2}", data.val_float.unwrap());
                        label.set_text(&text);
                    } else if data.command == OBDIICommandType::Maf {
                        let label = builder
                            .get_object::<gtk::Label>("MAFValue")
                            .expect("Can't find MAFValue in ui file.");
                        let text;
                        text = format!("{:3.2}", data.val_float.unwrap());
                        label.set_text(&text);

                        obdii_data
                            .borrow_mut()
                            .maf
                            .push_front(data.val_float.unwrap());
                        if obdii_data.borrow().maf.len() > obdii::VECTOR_LEN {
                            obdii_data.borrow_mut().maf.pop_back();
                        }

                        let chart = builder
                            .get_object::<gtk::DrawingArea>("OBDIIChartTwo")
                            .expect("Can't find OBDIIChartTwo in ui file.");

                        chart.queue_draw();
                    } else if data.command == OBDIICommandType::CoolantTemp {
                        let label = builder
                            .get_object::<gtk::Label>("CoolantTempValue")
                            .expect("Can't find CoolantTempValue in ui file.");
                        let text;
                        text = format!("{:3}", data.val_long.unwrap());
                        label.set_text(&text);
                    } else if data.command == OBDIICommandType::IntakeTemp {
                        let label = builder
                            .get_object::<gtk::Label>("IntakeTempValue")
                            .expect("Can't find IntakeTempValue in ui file.");
                        let text;
                        text = format!("{:3}", data.val_long.unwrap());
                        label.set_text(&text);
                    } else if data.command == OBDIICommandType::ShortFuelT1 {
                        let label = builder
                            .get_object::<gtk::Label>("ShortFuelB1Value")
                            .expect("Can't find ShortFuelB1Value in ui file.");
                        let text;
                        text = format!("{:3}", data.val_float.unwrap());
                        label.set_text(&text);
                    } else if data.command == OBDIICommandType::LongFuelT1 {
                        let label = builder
                            .get_object::<gtk::Label>("LongFuelB1Value")
                            .expect("Can't find LongFuelB1Value in ui file.");
                        let text;
                        text = format!("{:3}", data.val_float.unwrap());
                        label.set_text(&text);
                    } else if data.command == OBDIICommandType::FuelStatus {
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => return glib::source::Continue(true),
                _ => return glib::source::Continue(false),
            }
        }
        glib::source::Continue(true)
    }

    pub fn map_update_idle_thread(
        &self,
        location_rx: &std::sync::mpsc::Receiver<(f64, f64, i32, Option<bool>)>,
        map_wrapper: &mut MapWrapper,
    ) -> glib::source::Continue {
        let timeout = Duration::new(0, 100);
        let rec = location_rx.recv_timeout(timeout);
        match rec {
            Ok((lat, lon, status, neg)) => {
                map_wrapper.point.set_location(lat, lon);

                if self.start_on_track.lock().unwrap().get() {
                    map_wrapper.path_layer.remove_all();
                    self.start_on_track.lock().unwrap().set(false);
                }

                if self.on_track.lock().unwrap().get() {
                    let point_colour =
                        champlain::clutter_colour::ClutterColor::new(255, 60, 0, 255);
                    map_wrapper.point.set_colour(point_colour);

                    let colour = match neg {
                        Some(n) => {
                            if n {
                                println!("NegativeDiff");
                                champlain::clutter_colour::ClutterColor::new(204, 60, 0, 255)
                            } else {
                                println!("PositiveDiff");
                                champlain::clutter_colour::ClutterColor::new(0, 153, 76, 255)
                            }
                        }
                        None => champlain::clutter_colour::ClutterColor::new(0, 0, 255, 255),
                    };

                    map_wrapper.path_layer.set_stroke_colour(colour);

                    let mut coord = champlain::coordinate::ChamplainCoordinate::new_full(lat, lon);
                    map_wrapper.path_layer.add_node(coord.borrow_mut_location());
                } else {
                    crate::utils::set_point_colour(&mut map_wrapper.point, status);
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
