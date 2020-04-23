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
use crate::drive::course::{Course, MapWrapper};
use crate::drive::imu;
use crate::drive::obdii;
use crate::drive::prepare;
use crate::drive::threading::Threading;
use crate::drive::threading::ThreadingRef;
use gtk;
use gtk::prelude::*;
use gtk::ResponseType;
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

        thread_info.gpsd_thread(&mut course_info);
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

        thread_info.time_update_idle_thread(&times_rx, builder)
    });

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    let display_weak = DisplayRef::downgrade(&display);
    gtk::timeout_add(10, move || {
        let thread_info = upgrade_weak!(thread_info_weak, glib::source::Continue(false));
        let display = upgrade_weak!(display_weak, glib::source::Continue(false));

        let builder = display.builder.clone();

        thread_info.obdii_update_idle_thread(&obdii_rx, builder)
    });

    let imu_area: gtk::DrawingArea = builder
        .get_object("AccelDrawingArea")
        .expect("Couldn't find AccelDrawingArea in ui file.");

    let thread_info_weak = ThreadingRef::downgrade(&thread_info);
    imu_area.connect_draw(move |me, ctx| {
        let thread_info = upgrade_weak!(thread_info_weak, glib::signal::Inhibit(true));
        thread_info.imu_draw_idle_thread(&imu_rx, me, ctx)
    });

    gtk::timeout_add(35, move || {
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
        if response == ResponseType::Accept {
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

    #[allow(clippy::redundant_clone)]
    let thread_info_clone = thread_info.clone();
    gtk::timeout_add(10, move || {
        let thread_info = ThreadingRef::downgrade(&thread_info_clone)
            .upgrade()
            .unwrap();

        if thread_info.close.lock().unwrap().get() {
            return glib::source::Continue(false);
        }

        thread_info.map_update_idle_thread(&location_rx, &map_wrapper)
    });

    drive_page.show_all();
}
