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

use gtk::prelude::*;

pub fn button_press_event(display: DisplayRef) {
    let builder = display.builder.clone();

    let stack = builder
        .get_object::<gtk::Stack>("MainStack")
        .expect("Can't find MainStack in ui file.");

    stack.set_visible_child_name("RecordPage");

    let record_page = builder
        .get_object::<gtk::Paned>("RecordPage")
        .expect("Can't find RecordPage in ui file.");

    let champlain_gtk = champlain::gtk_embed::new();
    let _champlain_view =
        champlain::gtk_embed::get_view(champlain_gtk.clone()).expect("Unable to get ChamplainView");

    record_page.pack1(&champlain_gtk, true, true);
}
