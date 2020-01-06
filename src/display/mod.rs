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

#[macro_use]
use crate::utils;

use std::process;

use gtk::prelude::*;
use gio::prelude::*;

use std::sync::Arc;

use crate::track;

use crate::track::line;
use crate::track::record;

pub struct Display {
    main_window: gtk::ApplicationWindow,
    pub builder: gtk::Builder,
}

// We use Arc to avoid it being dropped
pub type DisplayRef = Arc<Display>;

impl Display {
    pub fn new(gtk_app: &gtk::Application) -> DisplayRef {
        let builder = gtk::Builder::new();

        let glade_src = include_str!("DashSight.glade");
        builder
            .add_from_string(glade_src)
            .expect("Couldn't add DashSight.glade from string");
        let glade_src = include_str!("StartPage.glade");
        builder
            .add_from_string(glade_src)
            .expect("Couldn't add StartPage.glade from string");
        let glade_src = include_str!("RecordPage.glade");
        builder
            .add_from_string(glade_src)
            .expect("Couldn't add RecordPage.glade from string");


        let window: gtk::ApplicationWindow = builder
            .get_object("MainPage")
            .expect("Couldn't find MainPage in ui file.");
        window.set_application(Some(gtk_app));

        let stack = builder
            .get_object::<gtk::Stack>("MainStack")
            .expect("Can't find MainStack in ui file.");

        /* Setup the start page */
        let child = builder
            .get_object::<gtk::Box>("SplashImage")
            .expect("Can't find SplashImage in ui file.");
        stack.add_named(&child, "SplashImage");

        /* Setup the record page */
        let record_page: gtk::Paned = builder
            .get_object("RecordPage")
            .expect("Couldn't find RecordPage in ui file.");
        stack.add_named(&record_page, "RecordPage");

        stack.set_visible_child_name("SplashImage");
        window.show_all();

        DisplayRef::new(Self {main_window: window.clone(), builder: builder.clone()})
    }

    pub fn on_startup(gtk_app: &gtk::Application) {
        // Create application
        let display = Display::new(gtk_app);
        let builder = display.builder.clone();

        let display_weak = DisplayRef::downgrade(&display);
        gtk_app.connect_activate(move |_| {
            let _display = upgrade_weak!(display_weak);
        });

        /* Setup actions for start page */
        let record_button: gtk::Button = builder
            .get_object("RecordTrack")
            .expect("Couldn't get RecordTrack");

        let display_weak = DisplayRef::downgrade(&display);
        record_button.connect_clicked(move |_| {
            let display = upgrade_weak!(display_weak);
            track::record::button_press_event(display)
        });

        let drive_line_button: gtk::Button = builder
            .get_object("DriveLine")
            .expect("Couldn't get DriveLine");

        let display_weak = DisplayRef::downgrade(&display);
        drive_line_button.connect_clicked(move |_| {
            let display = upgrade_weak!(display_weak);
            track::line::button_press_event(display)
        });

        let close_button: gtk::Button = builder
            .get_object("Close")
            .expect("Couldn't get Close");

        // We use a strong reference here to make sure that Display isn't dropped
        let display_clone = display.clone();
        close_button.connect_clicked(move |_| {
            // Just do something here to make sure this isn't dropped
            let _display_weak = DisplayRef::downgrade(&display_clone).upgrade().unwrap();
            process::exit(0);
        });
    }
}
