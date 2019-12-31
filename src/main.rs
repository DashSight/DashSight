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

extern crate gtk;
extern crate gio;
extern crate gdk;

use gtk::prelude::*;
use gio::prelude::*;

use gtk::{Box, ButtonBox, Image};

use std::env;

mod track;

fn main() {
    let uiapp = gtk::Application::new(
        Some("org.alistair23.DashSight"),
        Default::default(),
        ).expect("Application::new failed");

    uiapp.connect_activate(|app| {
        let win = gtk::ApplicationWindow::new(app);

        win.fullscreen();
        win.set_title("DashSight");

        let button_box = ButtonBox::new(gtk::Orientation::Horizontal);

        let record_button_image = Image::new_from_file("RecordTrack.png");
        let record_button = gtk::Button::new_with_label("Record new track");
        gtk::Container::add(&button_box.clone().upcast::<gtk::Container>(), &record_button);
        gtk::Button::set_always_show_image(&record_button, true);
        gtk::Button::set_image(&record_button, Some(&record_button_image));
        record_button.connect_clicked(|_| {
            track::record::button_press_event()
        });

        let drive_button_image = Image::new_from_file("DriveTrack.png");
        let drive_button = gtk::Button::new_with_label("Drive a single line");
        gtk::Container::add(&button_box.clone().upcast::<gtk::Container>(), &drive_button);
        gtk::Button::set_always_show_image(&drive_button, true);
        gtk::Button::set_image(&drive_button, Some(&drive_button_image));
        drive_button.connect_clicked(|_| {
            track::line::button_press_event()
        });

        let close_button = gtk::Button::new_with_label("Close!");
        gtk::Container::add(&button_box.clone().upcast::<gtk::Container>(), &close_button);
        // close_button.connect_clicked(|_| {
        //     win.close();
        // });

        let main_page = Box::new(gtk::Orientation::Vertical, 0);

        let main_image = Image::new_from_file("SplashPage.png");
        Box::pack_start(&main_page,
                        &main_image,
                        true, true, 0);

        Box::pack_start(&main_page,
                        &button_box,
                        true, true, 0);

        gtk::Container::add(&win.clone().upcast::<gtk::Container>(), &main_page);

        win.show_all();
    });

    uiapp.run(&env::args().collect::<Vec<_>>());
}
