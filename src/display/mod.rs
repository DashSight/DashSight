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

 pub struct Display {
    main_window: gtk::ApplicationWindow,
    pub builder: gtk::Builder,
}

impl Display {
    pub fn new(gtk_app: &gtk::Application) -> Display {
        let builder = gtk::Builder::new();

        let glade_src = include_str!("DashSight.glade");
        builder
            .add_from_string(glade_src)
            .expect("Couldn't add from string");

        let window: gtk::ApplicationWindow = builder
            .get_object("MainPage")
            .expect("Couldn't find MainPage in ui file.");
        window.set_application(Some(gtk_app));

        window.fullscreen();
        window.set_title("DashSight");

        window.show_all();

    	Display {
    		main_window: window,
	    	builder
    	}
	}

    pub fn on_startup(gtk_app: &gtk::Application) {
        // Create application
        let _display = Display::new(gtk_app);
    }
}
