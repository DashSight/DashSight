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
use crate::drive::drive;
use crate::drive::read_track;
use gtk;
use gtk::prelude::*;
use std::cell::Cell;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::process;
use std::rc::Rc;
use std::vec::Vec;

pub struct TrackSelection {
    pub track_points: Cell<Vec<crate::drive::read_track::Coord>>,
    pub map_widget: gtk::Widget,
}

pub type TrackSelectionRef = Rc<TrackSelection>;

impl TrackSelection {
    fn new(champlain_widget: gtk::Widget) -> TrackSelectionRef {
        TrackSelectionRef::new(Self {
            track_points: Cell::new(Vec::new()),
            map_widget: champlain_widget,
        })
    }
}

fn file_picker_clicked(display: DisplayRef, track_sel_info: TrackSelectionRef) {
    let builder = display.builder.clone();
    let champlain_view = champlain::gtk_embed::get_view(track_sel_info.map_widget.clone()).unwrap();

    let file_picker_button = builder
        .get_object::<gtk::FileChooserButton>("LoadMapFileLoadButton")
        .expect("Can't find LoadMapFileLoadButton in ui file.");

    if let Some(filepath) = file_picker_button.get_filename() {
        let track_file = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(filepath);

        let reader = BufReader::new(track_file.unwrap());
        let track_points = read_track::get_long_and_lat(reader);

        let path_layer = champlain::path_layer::new();
        champlain::view::set_zoom_level(champlain_view, 17);
        champlain::view::center_on(champlain_view, track_points[0].lat, track_points[0].lon);

        for coord in track_points.iter() {
            let c_point = champlain::coordinate::new_full(coord.lat, coord.lon);
            champlain::path_layer::add_node(
                path_layer,
                champlain::coordinate::to_location(c_point),
            );
        }

        champlain::view::add_layer(champlain_view, champlain::path_layer::to_layer(path_layer));

        track_sel_info.track_points.replace(track_points);

        let forward_button = builder
            .get_object::<gtk::Button>("LoadMapForwardButton")
            .expect("Can't find LoadMapForwardButton in ui file.");

        forward_button.set_sensitive(true);
    }
}

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

    let track_sel_info = TrackSelection::new(champlain_widget.clone());

    let file_picker_button = builder
        .get_object::<gtk::FileChooserButton>("LoadMapFileLoadButton")
        .expect("Can't find LoadMapFileLoadButton in ui file.");

    let display_weak = DisplayRef::downgrade(&display);
    let track_sel_info_weak = TrackSelectionRef::downgrade(&track_sel_info);
    file_picker_button.connect_file_set(move |_| {
        let display = upgrade_weak!(display_weak);
        let track_sel_info = upgrade_weak!(track_sel_info_weak);
        file_picker_clicked(display, track_sel_info);
    });

    let forward_button = builder
        .get_object::<gtk::Button>("LoadMapForwardButton")
        .expect("Can't find LoadMapForwardButton in ui file.");

    let display_weak = DisplayRef::downgrade(&display);
    // We use a strong reference here to make sure that track_sel_info isn't dropped
    let track_sel_info_clone = track_sel_info.clone();
    forward_button.connect_clicked(move |_| {
        let display = upgrade_weak!(display_weak);
        let track_sel_info = TrackSelectionRef::downgrade(&track_sel_info_clone)
            .upgrade()
            .unwrap();

        map_frame.remove(&champlain_widget);

        drive::button_press_event(display, track_sel_info);
    });

    forward_button.set_sensitive(false);

    load_map_page.show_all();
}
