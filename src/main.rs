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
use gdk::prelude::*;

use gtk::{Box, Image};

use std::env;

fn main() {
    let uiapp = gtk::Application::new(
        Some("org.alistair23.DashSight"),
        Default::default(),
        ).expect("Application::new failed");

    uiapp.connect_activate(|app| {
        let win = gtk::ApplicationWindow::new(app);

        win.fullscreen();
        win.set_title("DashSight");

        let main_image = Image::new_from_file("SplashPage.png");

        let main_page = Box::new(gtk::Orientation::Vertical, 0);
		Box::pack_start(&main_page,
						&main_image,
						true, true, 0);

        win.show_all();
    });

    uiapp.run(&env::args().collect::<Vec<_>>());
}
