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
use crate::drive::prepare;
use std::cell::RefCell;

struct LapTime {
    min: u64,
    sec: u64,
    nsec: u64,
}

struct Course {
    track_file: RefCell<std::path::PathBuf>,
    times: Vec<LapTime>,
    last: LapTime,
    best: LapTime,
    worst: LapTime,
}

pub fn button_press_event(_display: DisplayRef, _track_sel_info: prepare::TrackSelectionRef) {}
