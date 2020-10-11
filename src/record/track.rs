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

use crate::display::DisplayRef;
use crate::record::info::MapWrapper;
use crate::record::info::RecordInfo;
use crate::record::info::RecordInfoRef;
use gtk::prelude::*;
use std::process;
use std::sync::mpsc;
use std::thread;

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
    let mut champlain_view = champlain::gtk_embed::get_view(champlain_widget.clone());

    champlain_view.set_kinetic_mode(true);
    champlain_view.set_zoom_on_double_click(true);
    champlain_view.set_zoom_level(5);
    champlain_view.set_reactive(true);

    let layer = champlain::marker_layer::new();
    champlain::marker_layer::to_layer(layer)
        .to_clutter_actor()
        .show();
    champlain_view.add_layer(&mut champlain::marker_layer::to_layer(layer));

    let point_colour = champlain::clutter_colour::new(100, 200, 255, 255);

    let point = champlain::point::ChamplainPoint::new_full(12.0, point_colour);
    champlain::marker_layer::add_marker(layer, point.to_champlain_marker());

    let path_layer = champlain::path_layer::ChamplainPathLayer::new();
    champlain_view.add_layer(&mut path_layer.to_layer());

    let map_frame = builder
        .get_object::<gtk::Frame>("RecordPageMapFrame")
        .expect("Can't find RecordPageMapFrame in ui file.");

    map_frame.add(&champlain_widget);

    record_page.pack1(&map_frame, true, true);

    let (location_tx, location_rx) = mpsc::channel::<(f64, f64)>();
    let rec_info = RecordInfo::new();
    let mut map_wrapper = MapWrapper::new(champlain_view, path_layer, point);
    let mut first_connect = true;

    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    glib::timeout_add_local(10, move || {
        let rec_info = upgrade_weak!(rec_info_weak, glib::source::Continue(false));

        if rec_info.close.lock().unwrap().get() {
            return glib::source::Continue(false);
        }

        rec_info.idle_thread(&location_rx, &mut map_wrapper, &mut first_connect)
    });

    let file_picker_button = builder
        .get_object::<gtk::Button>("RecordFileSaveButton")
        .expect("Can't find RecordFileSaveButton in ui file.");

    let display_weak = DisplayRef::downgrade(&display);
    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    file_picker_button.connect_clicked(move |_| {
        let display = upgrade_weak!(display_weak);
        let rec_info = upgrade_weak!(rec_info_weak);
        rec_info.file_picker_clicked(display);
    });

    let record_button = builder
        .get_object::<gtk::ToggleButton>("RecordButton")
        .expect("Can't find RecordButton in ui file.");

    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    record_button.connect_clicked(move |_| {
        let rec_info = upgrade_weak!(rec_info_weak);
        rec_info.record_button_clicked();
    });

    let rec_info_weak = RecordInfoRef::downgrade(&rec_info);
    let _handler = thread::spawn(move || {
        let rec_info = rec_info_weak.upgrade().unwrap();
        rec_info.run(location_tx)
    });

    let back_button = builder
        .get_object::<gtk::Button>("RecordBackButton")
        .expect("Can't find RecordBackButton in ui file.");

    // We use a strong reference here to make sure that rec_info isn't dropped
    #[allow(clippy::redundant_clone)]
    let rec_info_clone = rec_info.clone();
    back_button.connect_clicked(move |_| {
        let rec_info = RecordInfoRef::downgrade(&rec_info_clone).upgrade().unwrap();
        rec_info.close.lock().unwrap().set(true);

        // handler.join().unwrap();

        stack.set_visible_child_name("SplashImage");
    });

    record_page.show_all();
}
