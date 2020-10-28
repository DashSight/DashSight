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
use crate::drive::course::Segment;
use crate::drive::course::{Course, MapWrapper};
use crate::drive::gps;
use crate::drive::imu;
use crate::drive::obdii;
use crate::drive::prepare;
use crate::drive::read_track::Coord;
use crate::drive::temp;
use crate::drive::threading::Threading;
use crate::drive::threading::ThreadingRef;
use gtk::prelude::*;
use gtk::ResponseType;
use plotters::prelude::*;
use plotters_cairo::CairoBackend;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn button_press_event(display: DisplayRef, track_sel_info: prepare::TrackSelectionRef) {
    let builder = display.builder.clone();

    let stack = builder
        .get_object::<gtk::Stack>("MainStack")
        .expect("Can't find MainStack in ui file.");
    stack.set_visible_child_name("DrivePage");

    let drive_page = builder
        .get_object::<gtk::Notebook>("DriveNotebook")
        .expect("Can't find DriveNotebook in ui file.");

    let map_frame = builder
        .get_object::<gtk::Frame>("DriveMapFrame")
        .expect("Can't find DriveMapFrame in ui file.");
    map_frame.add(&track_sel_info.map_widget);

    let mut champlain_view = champlain::gtk_embed::get_view(track_sel_info.map_widget.clone());

    let track_points = track_sel_info.track_points.take();

    let (location_tx, location_rx) = mpsc::channel::<(f64, f64, i32, Option<bool>)>();
    let (elapsed_tx, elapsed_rx) = mpsc::channel::<Duration>();
    let (times_tx, times_rx) = mpsc::channel::<(Duration, Duration, Duration)>();
    let (time_diff_tx, time_diff_rx) = mpsc::channel::<(bool, Duration)>();
    let (obdii_tx, obdii_rx) = mpsc::channel::<obdii::OBDIIData>();
    let (imu_tx, imu_rx) = mpsc::channel::<(f64, f64, Option<f64>, Option<f64>)>();
    let (imu_page_tx, imu_page_rx) = mpsc::channel::<(f64, f64, Option<f64>, Option<f64>)>();
    let (temp_tx, temp_rx) = mpsc::channel::<Vec<f64>>();
    let thread_info = Threading::new();

    let window: gtk::ApplicationWindow = builder
        .get_object("MainPage")
        .expect("Couldn't find MainPage in ui file.");

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let _handler_gpsd = thread::spawn(move || {
        let thread_info = upgrade_weak!(thread_info_weak);
        let mut segments = Vec::new();

        for points in track_points {
            segments.push(Segment::new(
                Coord::new(
                    (&points).first().unwrap().lat,
                    (&points).first().unwrap().lon,
                    (&points).first().unwrap().head,
                ),
                Coord::new(
                    (&points).last().unwrap().lat,
                    (&points).last().unwrap().lon,
                    (&points).last().unwrap().head,
                ),
            ));
        }

        let mut course_info = Course::new(segments);

        gps::gpsd_thread(
            thread_info,
            elapsed_tx,
            times_tx,
            time_diff_tx,
            location_tx,
            &mut course_info,
        );
    });

    let mut track_name = track_sel_info.track_file.borrow().clone();
    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let _handler_obdii = thread::spawn(move || {
        let thread_info = upgrade_weak!(thread_info_weak);

        obdii::obdii_thread(thread_info, obdii_tx, &mut track_name).unwrap();
    });

    let mut track_name = track_sel_info.track_file.borrow().clone();
    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let _handler_imu = thread::spawn(move || {
        let thread_info = upgrade_weak!(thread_info_weak);

        imu::imu_thread(thread_info, imu_tx, imu_page_tx, &mut track_name);
    });

    let mut track_name = track_sel_info.track_file.borrow().clone();
    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let _handler_imu = thread::spawn(move || {
        let thread_info = upgrade_weak!(thread_info_weak);

        temp::temp_thread(thread_info, temp_tx, &mut track_name);
    });

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let display_weak = DisplayRef::downgrade(&display);
    glib::timeout_add_local(10, move || {
        let thread_info = upgrade_weak!(thread_info_weak, glib::source::Continue(false));
        let display = upgrade_weak!(display_weak, glib::source::Continue(false));

        let builder = display.builder.clone();

        if thread_info.close.lock().unwrap().get() {
            return glib::source::Continue(false);
        }

        thread_info.time_update_idle_thread(&elapsed_rx, &times_rx, &time_diff_rx, builder)
    });

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let display_weak = DisplayRef::downgrade(&display);
    let obdii_data = Rc::new(RefCell::new(obdii::OBDIIGraphData::new()));
    thread_info.set_cairo_graphs(&builder, &obdii_data);

    glib::timeout_add_local(10, move || {
        let thread_info = upgrade_weak!(thread_info_weak, glib::source::Continue(false));
        let display = upgrade_weak!(display_weak, glib::source::Continue(false));

        let builder = display.builder.clone();

        thread_info.obdii_update_idle_thread(&obdii_rx, builder, &obdii_data)
    });

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let display_weak = DisplayRef::downgrade(&display);
    glib::timeout_add_local(10, move || {
        let thread_info = upgrade_weak!(thread_info_weak, glib::source::Continue(false));
        let display = upgrade_weak!(display_weak, glib::source::Continue(false));

        let builder = display.builder.clone();

        if thread_info.close.lock().unwrap().get() {
            return glib::source::Continue(false);
        }

        thread_info.temp_update_idle_thread(&temp_rx, builder)
    });

    let imu_area: gtk::DrawingArea = builder
        .get_object("AccelDrawingArea")
        .expect("Couldn't find AccelDrawingArea in ui file.");

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let display_weak = DisplayRef::downgrade(&display);
    imu_area.connect_draw(move |me, ctx| {
        let thread_info = upgrade_weak!(thread_info_weak, glib::signal::Inhibit(true));
        let display = upgrade_weak!(display_weak, Inhibit(true));

        let builder = display.builder.clone();

        thread_info.imu_draw_idle_thread(&imu_rx, me, ctx, builder)
    });

    let imu_page_accel_area: gtk::DrawingArea = builder
        .get_object("IMUPageAcellDraw")
        .expect("Couldn't find IMUPageAcellDraw in ui file.");

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let display_weak = DisplayRef::downgrade(&display);
    imu_page_accel_area.connect_draw(move |me, ctx| {
        let thread_info = upgrade_weak!(thread_info_weak, glib::signal::Inhibit(true));
        let display = upgrade_weak!(display_weak, Inhibit(true));

        let builder = display.builder.clone();

        thread_info.imu_draw_idle_thread(&imu_page_rx, me, ctx, builder)
    });

    glib::timeout_add_local(imu::IMU_SAMPLE_FREQ as u32, move || {
        imu_area.queue_draw();
        imu_page_accel_area.queue_draw();

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
        if response == ResponseType::Accept {
            if let Some(filepath) = file_chooser.get_filename() {
                let mut time_file = thread_info.time_file.write().unwrap();
                *time_file = filepath;
                thread_info.serialise.lock().unwrap().set(true);
            }
        }
    });

    let calibrate_button = display
        .builder
        .get_object::<gtk::Button>("CalibrateOptionsPopOverSave")
        .expect("Can't find CalibrateOptionsPopOverSave in ui file.");

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    calibrate_button.connect_clicked(move |_| {
        let thread_info = upgrade_weak!(thread_info_weak);

        thread_info.calibrate.lock().unwrap().set(true);
    });

    let mut layer = champlain::marker_layer::ChamplainMarkerLayer::new();
    layer.borrow_mut_actor().show();
    champlain_view.add_layer(layer.borrow_mut_layer());

    let point_colour = champlain::clutter_colour::ClutterColor::new(100, 200, 255, 255);

    let mut point = champlain::point::ChamplainPoint::new_full(12.0, point_colour);
    layer.add_marker(point.borrow_mut_marker());

    let mut pos_path_layer = champlain::path_layer::ChamplainPathLayer::new();
    champlain_view.add_layer(pos_path_layer.borrow_mut_layer());
    let colour = champlain::clutter_colour::ClutterColor::new(204, 60, 0, 255);
    pos_path_layer.set_stroke_colour(colour);
    pos_path_layer.set_visible(true);

    let mut neg_path_layer = champlain::path_layer::ChamplainPathLayer::new();
    champlain_view.add_layer(neg_path_layer.borrow_mut_layer());
    let colour = champlain::clutter_colour::ClutterColor::new(0, 153, 76, 255);
    neg_path_layer.set_stroke_colour(colour);
    neg_path_layer.set_visible(true);

    layer.show_all_markers();

    let mut map_wrapper = MapWrapper::new(pos_path_layer, neg_path_layer, point);

    #[allow(clippy::redundant_clone)]
    let thread_info_clone = thread_info.clone();
    glib::timeout_add_local(10, move || {
        let thread_info = ThreadingRef::downgrade(&thread_info_clone)
            .upgrade()
            .unwrap();

        if thread_info.close.lock().unwrap().get() {
            layer.remove_all();
            return glib::source::Continue(false);
        }

        thread_info.map_update_idle_thread(&location_rx, &mut map_wrapper)
    });

    drive_page.show_all();
}

impl Threading {
    pub fn set_cairo_graphs(
        &self,
        builder: &gtk::Builder,
        obdii_data: &Rc<RefCell<obdii::OBDIIGraphData>>,
    ) {
        let chart = builder
            .get_object::<gtk::DrawingArea>("OBDIIChartOne")
            .expect("Can't find OBDIIChartOne in ui file.");

        let obdii_data_cloned = obdii_data.clone();

        chart.connect_draw(move |me, cr| {
            let width = me.get_allocated_width() as f64 * 0.07;
            let height = me.get_allocated_width() as f64 * 0.07;

            let root = CairoBackend::new(cr, (500, 500))
                .unwrap()
                .into_drawing_area();

            let mut chart = ChartBuilder::on(&root)
                .margin(10)
                .caption("RPM", ("sans-serif", 30).into_font())
                .x_label_area_size(width as u32)
                .y_label_area_size(height as u32)
                .build_cartesian_2d(0..100 as u32, 0f64..15000f64)
                .unwrap();

            chart.configure_mesh().draw().unwrap();

            chart
                .draw_series(AreaSeries::new(
                    obdii_data_cloned
                        .borrow_mut()
                        .rpm
                        .iter()
                        .enumerate()
                        .map(|(x, y)| (x as u32, *y)),
                    0.0,
                    &BLUE.mix(0.2),
                ))
                .unwrap();

            Inhibit(true)
        });

        let chart = builder
            .get_object::<gtk::DrawingArea>("OBDIIChartTwo")
            .expect("Can't find OBDIIChartTwo in ui file.");

        let obdii_data_cloned = obdii_data.clone();

        chart.connect_draw(move |me, cr| {
            let width = me.get_allocated_width() as f64 * 0.07;
            let height = me.get_allocated_width() as f64 * 0.07;

            let root = CairoBackend::new(cr, (500, 500))
                .unwrap()
                .into_drawing_area();

            let mut chart = ChartBuilder::on(&root)
                .margin(10)
                .caption("MAF (%)", ("sans-serif", 30).into_font())
                .x_label_area_size(width as u32)
                .y_label_area_size(height as u32)
                .build_cartesian_2d(0..100 as u32, 0f64..100f64)
                .unwrap();

            chart.configure_mesh().draw().unwrap();

            chart
                .draw_series(AreaSeries::new(
                    obdii_data_cloned
                        .borrow_mut()
                        .maf
                        .iter()
                        .enumerate()
                        .map(|(x, y)| (x as u32, *y)),
                    0.0,
                    &RED.mix(0.2),
                ))
                .unwrap();

            Inhibit(true)
        });

        let chart = builder
            .get_object::<gtk::DrawingArea>("OBDIIChartThree")
            .expect("Can't find OBDIIChartThree in ui file.");

        let obdii_data_cloned = obdii_data.clone();

        chart.connect_draw(move |me, cr| {
            let width = me.get_allocated_width() as f64 * 0.07;
            let height = me.get_allocated_width() as f64 * 0.07;

            let root = CairoBackend::new(cr, (500, 500))
                .unwrap()
                .into_drawing_area();

            let mut chart = ChartBuilder::on(&root)
                .margin(10)
                .caption("Throtle (%)", ("sans-serif", 30).into_font())
                .x_label_area_size(width as u32)
                .y_label_area_size(height as u32)
                .build_cartesian_2d(0..100 as u32, 0f64..100f64)
                .unwrap();

            chart.configure_mesh().draw().unwrap();

            chart
                .draw_series(AreaSeries::new(
                    obdii_data_cloned
                        .borrow_mut()
                        .throttle
                        .iter()
                        .enumerate()
                        .map(|(x, y)| (x as u32, *y)),
                    0.0,
                    &GREEN.mix(0.2),
                ))
                .unwrap();

            Inhibit(true)
        });

        let chart = builder
            .get_object::<gtk::DrawingArea>("OBDIIChartFour")
            .expect("Can't find OBDIIChartFour in ui file.");

        let obdii_data_cloned = obdii_data.clone();

        chart.connect_draw(move |me, cr| {
            let width = me.get_allocated_width() as f64 * 0.07;
            let height = me.get_allocated_width() as f64 * 0.07;

            let root = CairoBackend::new(cr, (500, 500))
                .unwrap()
                .into_drawing_area();

            let mut chart = ChartBuilder::on(&root)
                .margin(10)
                .caption("Load (%)", ("sans-serif", 30).into_font())
                .x_label_area_size(width as u32)
                .y_label_area_size(height as u32)
                .build_cartesian_2d(0..100 as u32, 0f64..100f64)
                .unwrap();

            chart.configure_mesh().draw().unwrap();

            chart
                .draw_series(AreaSeries::new(
                    obdii_data_cloned
                        .borrow_mut()
                        .load
                        .iter()
                        .enumerate()
                        .map(|(x, y)| (x as u32, *y)),
                    0.0,
                    &YELLOW.mix(0.2),
                ))
                .unwrap();

            Inhibit(true)
        });
    }
}
