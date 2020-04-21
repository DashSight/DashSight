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

use crate::drive::read_track::Coord;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct Course {
    pub times: Vec<Duration>,
    pub last: Duration,
    pub best: Duration,
    pub worst: Duration,
    pub start: Coord,
    pub finish: Coord,
}

impl Course {
    pub fn new(start_lat: f64, start_lon: f64, finish_lat: f64, finish_lon: f64) -> Course {
        Course {
            times: Vec::new(),
            last: Duration::new(0, 0),
            best: Duration::new(0, 0),
            worst: Duration::new(0, 0),
            start: Coord {
                lat: start_lat,
                lon: start_lon,
            },
            finish: Coord {
                lat: finish_lat,
                lon: finish_lon,
            },
        }
    }
}

pub struct MapWrapper {
    pub path_layer: *mut champlain::path_layer::ChamplainPathLayer,
    pub point: *mut champlain::clutter::ClutterActor,
}

impl MapWrapper {
    pub fn new(
        path_layer: *mut champlain::path_layer::ChamplainPathLayer,
        champlain_point: *mut champlain::clutter::ClutterActor,
    ) -> MapWrapper {
        MapWrapper {
            path_layer,
            point: champlain_point,
        }
    }
}
