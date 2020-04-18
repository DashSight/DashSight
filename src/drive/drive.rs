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
use crate::drive::imu;
use crate::drive::obdii;
use crate::drive::obdii::OBDIICommandType;
use crate::drive::prepare;
use crate::drive::read_track::Coord;
use crate::utils::lat_lon_comp;
use gpsd_proto::handshake;
use gtk;
use gtk::prelude::*;
use gtk::ResponseType;
use serde::{Deserialize, Serialize};
use serde_json;
use std::cell::Cell;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Serialize, Deserialize)]
struct Course {
    times: Vec<Duration>,
    last: Duration,
    best: Duration,
    worst: Duration,
    start: Coord,
    finish: Coord,
}

impl Course {
    fn new(start_lat: f64, start_lon: f64, finish_lat: f64, finish_lon: f64) -> Course {
        Course {
            times: Vec::new(),
            last: Duration::new(0, 0),
            best: Duration::new(0, 0),
            worst: Duration::new(0, 0),
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

pub struct Threading {
    pub lap_start: RefCell<std::time::SystemTime>,
    pub close: Mutex<Cell<bool>>,
    pub on_track: Mutex<Cell<bool>>,
    pub change_colour: Mutex<Cell<bool>>,
    pub no_track: Mutex<Cell<bool>>,
    serialise: Mutex<Cell<bool>>,
    time_file: RefCell<std::path::PathBuf>,
    pub location_tx: std::sync::mpsc::Sender<(f64, f64)>,
    pub times_tx: std::sync::mpsc::Sender<(Duration, Duration, Duration)>,
    pub obdii_tx: std::sync::mpsc::Sender<obdii::OBDIIData>,
    pub imu_tx: std::sync::mpsc::Sender<(f64, f64)>,
}

pub type ThreadingRef = Arc<Threading>;

impl Threading {
    fn new(
        location_tx: std::sync::mpsc::Sender<(f64, f64)>,
        times_tx: std::sync::mpsc::Sender<(Duration, Duration, Duration)>,
        obdii_tx: std::sync::mpsc::Sender<obdii::OBDIIData>,
        imu_tx: std::sync::mpsc::Sender<(f64, f64)>,
    ) -> ThreadingRef {
        ThreadingRef::new(Self {
            lap_start: RefCell::new(SystemTime::now()),
            close: Mutex::new(Cell::new(false)),
            on_track: Mutex::new(Cell::new(false)),
            change_colour: Mutex::new(Cell::new(false)),
            no_track: Mutex::new(Cell::new(false)),
            serialise: Mutex::new(Cell::new(false)),
            time_file: RefCell::new(PathBuf::new()),
            location_tx: location_tx,
            times_tx: times_tx,
            obdii_tx: obdii_tx,
            imu_tx,
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
                if thread_info.close.lock().unwrap().get() {
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

    while !thread_info.close.lock().unwrap().get() {
        let msg = crate::utils::get_gps_lat_lon(&mut reader);

        match msg {
            Ok((lat, lon, _alt, _time, _speed)) => {
                thread_info.location_tx.send((lat, lon)).unwrap();

                if !thread_info.on_track.lock().unwrap().get()
                    && lat_lon_comp(lat, lon, course_info.start.lat, course_info.start.lon)
                {
                    thread_info.lap_start.replace(SystemTime::now());
                    thread_info.on_track.lock().unwrap().set(true);
                    thread_info.change_colour.lock().unwrap().set(true);
                }

                if thread_info.on_track.lock().unwrap().get()
                    && lat_lon_comp(lat, lon, course_info.finish.lat, course_info.finish.lon)
                {
                    thread_info.on_track.lock().unwrap().set(false);
                    thread_info.change_colour.lock().unwrap().set(true);

                    match thread_info.lap_start.borrow().elapsed() {
                        Ok(elapsed) => {
                            course_info.times.push(elapsed);
                            course_info.last = elapsed;
                            course_info.times.sort_unstable();
                            match course_info.times.last() {
                                Some(worst) => course_info.worst = worst.clone(),
                                _ => {}
                            }
                            match course_info.times.first() {
                                Some(best) => course_info.best = best.clone(),
                                _ => {}
                            }
                            thread_info
                                .times_tx
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

        if thread_info.serialise.lock().unwrap().get() {
            let mut track_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(thread_info.time_file.borrow().clone());

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

fn time_update_idle_thread(
    times_rx: &std::sync::mpsc::Receiver<(Duration, Duration, Duration)>,
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

fn obdii_update_idle_thread(
    obdii_rx: &std::sync::mpsc::Receiver<obdii::OBDIIData>,
    builder: gtk::Builder,
    _thread_info: ThreadingRef,
) -> glib::source::Continue {
    let timeout = Duration::new(0, 100);
    let rec = obdii_rx.recv_timeout(timeout);
    match rec {
        Ok(data) => {
            if data.command == OBDIICommandType::Rpm {
            } else if data.command == OBDIICommandType::Throttle {
                let bar = builder
                    .get_object::<gtk::ProgressBar>("ThrottleBar")
                    .expect("Can't find ThrottleBar in ui file.");
                unsafe {
                    bar.set_fraction(data.val.float / 100.0);
                }
            } else if data.command == OBDIICommandType::EngineLoad {
                let bar = builder
                    .get_object::<gtk::ProgressBar>("LoadBar")
                    .expect("Can't find LoadBar in ui file.");
                unsafe {
                    bar.set_fraction(data.val.float / 100.0);
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

fn map_update_idle_thread(
    location_rx: &std::sync::mpsc::Receiver<(f64, f64)>,
    map_wrapper: &MapWrapper,
    thread_info: ThreadingRef,
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

            if thread_info.change_colour.lock().unwrap().get() {
                if thread_info.on_track.lock().unwrap().get() {
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
                thread_info.change_colour.lock().unwrap().set(false);
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

fn draw_imu(
    imu_rx: &std::sync::mpsc::Receiver<(f64, f64)>,
    me: &gtk::DrawingArea,
    ctx: &cairo::Context,
) -> glib::signal::Inhibit {
    println!("Starting redraw");
    let timeout = Duration::new(0, 100);
    let rec = imu_rx.recv_timeout(timeout);

    let width = me.get_allocated_width() as f64;
    let height = me.get_allocated_width() as f64 * 0.7;

    ctx.set_source_rgba(0.0, 0.0, 0.0, 0.9);
    ctx.set_line_width(0.2);

    // draw circles
    ctx.arc(0.5 * width, 0.5 * height, 0.1 * height, 0.0, 3.1415 * 2.);
    ctx.stroke();
    ctx.arc(0.5 * width, 0.5 * height, 0.2 * height, 0.0, 3.1415 * 2.);
    ctx.stroke();
    ctx.arc(0.5 * width, 0.5 * height, 0.3 * height, 0.0, 3.1415 * 2.);
    ctx.stroke();
    ctx.arc(0.5 * width, 0.5 * height, 0.4 * height, 0.0, 3.1415 * 2.);
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
            println!(" Adding dot: {}, {}", x_accel, y_accel);
            ctx.set_source_rgba(0.0, 148.0 / 255.0, 1.0, 1.0);

            ctx.arc(
                (0.5 * width) + x_accel,
                (0.5 * height) + y_accel,
                5.0,
                0.0,
                3.1415 * 2.,
            );
            ctx.fill();

            println!("  Done");
            Inhibit(false)
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            println!("  Timeout");
            Inhibit(false)
        }
        _ => Inhibit(true),
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

    let track_points = track_sel_info.track_points.take();

    let (location_tx, location_rx) = mpsc::channel::<(f64, f64)>();
    let (times_tx, times_rx) = mpsc::channel::<(Duration, Duration, Duration)>();
    let (obdii_tx, obdii_rx) = mpsc::channel::<obdii::OBDIIData>();
    let (imu_tx, imu_rx) = mpsc::channel::<(f64, f64)>();
    let thread_info = Threading::new(location_tx, times_tx, obdii_tx, imu_tx);

    let window: gtk::ApplicationWindow = builder
        .get_object("MainPage")
        .expect("Couldn't find MainPage in ui file.");

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let _handler_gpsd = thread::spawn(move || {
        let thread_info = upgrade_weak!(thread_info_weak);

        let mut course_info = Course::new(
            (&track_points).first().unwrap().lat,
            (&track_points).first().unwrap().lon,
            (&track_points).last().unwrap().lat,
            (&track_points).last().unwrap().lon,
        );

        gpsd_thread(&mut course_info, thread_info);
    });

    let mut track_name = track_sel_info.track_file.borrow().clone();
    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let _handler_obdii = thread::spawn(move || {
        let thread_info = upgrade_weak!(thread_info_weak);

        obdii::obdii_thread(thread_info, &mut track_name).unwrap();
    });

    let mut track_name = track_sel_info.track_file.borrow().clone();
    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let _handler_imu = thread::spawn(move || {
        let thread_info = upgrade_weak!(thread_info_weak);

        imu::imu_thread(thread_info, &mut track_name);
    });

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let display_weak = DisplayRef::downgrade(&display);
    gtk::timeout_add(10, move || {
        let thread_info = upgrade_weak!(thread_info_weak, glib::source::Continue(false));
        let display = upgrade_weak!(display_weak, glib::source::Continue(false));

        let builder = display.builder.clone();

        if thread_info.close.lock().unwrap().get() {
            return glib::source::Continue(false);
        }

        time_update_idle_thread(&times_rx, builder, thread_info)
    });

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let display_weak = DisplayRef::downgrade(&display);
    gtk::timeout_add(10, move || {
        let thread_info = upgrade_weak!(thread_info_weak, glib::source::Continue(false));
        let display = upgrade_weak!(display_weak, glib::source::Continue(false));

        let builder = display.builder.clone();

        obdii_update_idle_thread(&obdii_rx, builder, thread_info)
    });

    let imu_area: gtk::DrawingArea = builder
        .get_object("AccelDrawingArea")
        .expect("Couldn't find AccelDrawingArea in ui file.");

    imu_area.connect_draw(move |me, ctx| draw_imu(&imu_rx, me, ctx));

    gtk::timeout_add(10, move || {
        imu_area.queue_draw();

        glib::source::Continue(true)
    });

    let close_button = builder
        .get_object::<gtk::Button>("DriveOptionsPopOverClose")
        .expect("Can't find DriveOptionsPopOverClose in ui file.");

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    close_button.connect_clicked(move |_| {
        let thread_info = upgrade_weak!(thread_info_weak);
        thread_info.close.lock().unwrap().set(true);

        stack.set_visible_child_name("SplashImage");
    });

    let save_button = builder
        .get_object::<gtk::Button>("DriveOptionsPopOverSave")
        .expect("Can't find DriveOptionsPopOverClose in ui file.");

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    save_button.connect_clicked(move |_| {
        let thread_info = upgrade_weak!(thread_info_weak);

        let file_chooser = gtk::FileChooserNative::new(
            Some("Save times as"),
            Some(&window),
            gtk::FileChooserAction::Save,
            Some("Save"),
            Some("Close"),
        );

        let response = file_chooser.run();
        if ResponseType::from(response) == ResponseType::Accept {
            if let Some(filepath) = file_chooser.get_filename() {
                thread_info.time_file.replace(filepath);
                thread_info.serialise.lock().unwrap().set(true);
            }
        }
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
    gtk::timeout_add(10, move || {
        let thread_info = ThreadingRef::downgrade(&thread_info_clone)
            .upgrade()
            .unwrap();

        if thread_info.close.lock().unwrap().get() {
            return glib::source::Continue(false);
        }

        map_update_idle_thread(&location_rx, &map_wrapper, thread_info)
    });

    drive_page.show_all();
}
