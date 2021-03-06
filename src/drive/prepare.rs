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
use crate::drive::display;
use crate::drive::read_track;
use crate::utils::genereate_polygon;
use gtk::prelude::*;
use std::cell::Cell;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::path::PathBuf;
use std::process;
use std::rc::Rc;
use std::vec::Vec;

pub struct TrackSelection {
    pub track_file: RefCell<std::path::PathBuf>,
    pub track_points: Cell<Vec<Vec<crate::drive::read_track::Coord>>>,
    pub map_widget: gtk::Widget,
    map_layers: Cell<Vec<champlain::path_layer::ChamplainPathLayer>>,
}

pub type TrackSelectionRef = Rc<TrackSelection>;

impl TrackSelection {
    fn new(champlain_widget: gtk::Widget) -> TrackSelectionRef {
        TrackSelectionRef::new(Self {
            track_file: RefCell::new(PathBuf::new()),
            track_points: Cell::new(Vec::new()),
            map_widget: champlain_widget,
            map_layers: Cell::new(Vec::new()),
        })
    }

    fn file_picker_clicked(&self, display: DisplayRef) {
        let builder = display.builder.clone();
        let mut champlain_view = champlain::gtk_embed::get_view(self.map_widget.clone());

        let file_picker_button = builder
            .get_object::<gtk::FileChooserButton>("LoadMapFileLoadButton")
            .expect("Can't find LoadMapFileLoadButton in ui file.");

        if let Some(filepath) = file_picker_button.get_filename() {
            let track_file = OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .open(&filepath);

            self.track_file.replace(filepath);

            let reader = BufReader::new(track_file.unwrap());
            let track_points = read_track::get_long_and_lat(reader);

            // Remove all current layers
            let mut new_map_layers = self.map_layers.take();
            while !new_map_layers.is_empty() {
                new_map_layers.pop().unwrap().remove_all();
            }

            champlain_view.set_zoom_level(17);
            champlain_view.center_on(
                track_points.first().unwrap().first().unwrap().lat,
                track_points.first().unwrap().first().unwrap().lon,
            );

            // Add the track layer
            let mut path_layer = champlain::path_layer::ChamplainPathLayer::new();

            for points in track_points.iter() {
                for coord in points.iter() {
                    let mut c_point =
                        champlain::coordinate::ChamplainCoordinate::new_full(coord.lat, coord.lon);
                    path_layer.add_node(c_point.borrow_mut_location());
                }
            }

            champlain_view.add_layer(path_layer.borrow_mut_layer());
            let mut new_map_layers = self.map_layers.take();
            new_map_layers.push(path_layer);
            self.map_layers.replace(new_map_layers);

            // Add the start polygons
            for segment in &track_points {
                let mut path_layer = champlain::path_layer::ChamplainPathLayer::new();

                let start_poly = genereate_polygon(
                    segment.first().unwrap().lat,
                    segment.first().unwrap().lon,
                    segment.first().unwrap().head.unwrap_or(0.0),
                );

                path_layer.set_stroke_colour(champlain::clutter_colour::ClutterColor::new(
                    255, 255, 255, 150,
                ));

                for coord in start_poly.points().iter() {
                    let mut c_point =
                        champlain::coordinate::ChamplainCoordinate::new_full(coord[0], coord[1]);
                    path_layer.add_node(c_point.borrow_mut_location());
                }
                // Add the first point again to create a closed shape
                let mut c_point = champlain::coordinate::ChamplainCoordinate::new_full(
                    start_poly.points()[0][0],
                    start_poly.points()[0][1],
                );
                path_layer.add_node(c_point.borrow_mut_location());

                champlain_view.add_layer(path_layer.borrow_mut_layer());
                let mut new_map_layers = self.map_layers.take();
                new_map_layers.push(path_layer);
                self.map_layers.replace(new_map_layers);
            }

            // Add the end polygon
            let mut path_layer = champlain::path_layer::ChamplainPathLayer::new();

            let end_poly = genereate_polygon(
                track_points.last().unwrap().last().unwrap().lat,
                track_points.last().unwrap().last().unwrap().lon,
                track_points
                    .last()
                    .unwrap()
                    .last()
                    .unwrap()
                    .head
                    .unwrap_or(0.0),
            );

            path_layer
                .set_stroke_colour(champlain::clutter_colour::ClutterColor::new(0, 0, 0, 150));

            for coord in end_poly.points().iter() {
                let mut c_point =
                    champlain::coordinate::ChamplainCoordinate::new_full(coord[0], coord[1]);
                path_layer.add_node(c_point.borrow_mut_location());
            }
            // Add the first point again to create a closed shape
            let mut c_point = champlain::coordinate::ChamplainCoordinate::new_full(
                end_poly.points()[0][0],
                end_poly.points()[0][1],
            );
            path_layer.add_node(c_point.borrow_mut_location());

            champlain_view.add_layer(path_layer.borrow_mut_layer());
            let mut new_map_layers = self.map_layers.take();
            new_map_layers.push(path_layer);
            self.map_layers.replace(new_map_layers);

            self.track_points.replace(track_points);

            let forward_button = builder
                .get_object::<gtk::Button>("LoadMapForwardButton")
                .expect("Can't find LoadMapForwardButton in ui file.");

            forward_button.set_sensitive(true);
        }
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
    let mut champlain_view = champlain::gtk_embed::get_view(champlain_widget.clone());

    champlain_view.set_kinetic_mode(true);
    champlain_view.set_zoom_on_double_click(true);
    champlain_view.set_zoom_level(5);
    champlain_view.set_reactive(true);

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
        track_sel_info.file_picker_clicked(display);
    });

    let forward_button = builder
        .get_object::<gtk::Button>("LoadMapForwardButton")
        .expect("Can't find LoadMapForwardButton in ui file.");

    let display_weak = DisplayRef::downgrade(&display);
    // We use a strong reference here to make sure that track_sel_info isn't dropped
    #[allow(clippy::redundant_clone)]
    let track_sel_info_clone = track_sel_info.clone();
    forward_button.connect_clicked(move |_| {
        let display = upgrade_weak!(display_weak);
        let track_sel_info = TrackSelectionRef::downgrade(&track_sel_info_clone)
            .upgrade()
            .unwrap();

        map_frame.remove(&champlain_widget);

        display::button_press_event(display, track_sel_info);
    });

    forward_button.set_sensitive(false);

    load_map_page.show_all();
}
