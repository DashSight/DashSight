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
use gtk;
use gtk::prelude::*;
use std::process;

pub fn button_press_event(display: DisplayRef) {
    let builder = display.builder.clone();

    let stack = builder
        .get_object::<gtk::Stack>("MainStack")
        .expect("Can't find MainStack in ui file.");

    stack.set_visible_child_name("LoadMapPage");

    let load_map_page = builder
        .get_object::<gtk::Paned>("LoadMapPage")
        .expect("Can't find LoadMapPage in ui file.");

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
        .get_object::<gtk::Frame>("LoadMapPageMapFrame")
        .expect("Can't find LoadMapPageMapFrame in ui file.");

    map_frame.add(&champlain_widget);

    load_map_page.pack1(&map_frame, true, true);

    load_map_page.show_all();
}
