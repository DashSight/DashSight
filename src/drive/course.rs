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
pub struct Segment {
    pub start: Coord,
    pub finish: Coord,
}

impl Segment {
    pub fn new(start: Coord, finish: Coord) -> Self {
        Self { start, finish }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Course {
    pub times: Vec<Duration>,
    pub last: Duration,
    pub best: Duration,
    pub best_times: Vec<Vec<(Coord, Duration)>>,
    pub last_location_time: Option<Duration>,
    pub worst: Duration,
    pub segments: Vec<Segment>,
}

impl Course {
    pub fn new(segments: Vec<Segment>) -> Course {
        Course {
            times: Vec::new(),
            last: Duration::new(0, 0),
            best: Duration::new(0, 0),
            best_times: Vec::new(),
            last_location_time: None,
            worst: Duration::new(0, 0),
            segments,
        }
    }
}

pub struct MapWrapper {
    pub pos_path_layer: champlain::path_layer::ChamplainPathLayer,
    pub neg_path_layer: champlain::path_layer::ChamplainPathLayer,
    pub point: champlain::point::ChamplainPoint,
}

impl MapWrapper {
    pub fn new(
        pos_path_layer: champlain::path_layer::ChamplainPathLayer,
        neg_path_layer: champlain::path_layer::ChamplainPathLayer,
        champlain_point: champlain::point::ChamplainPoint,
    ) -> MapWrapper {
        MapWrapper {
            pos_path_layer,
            neg_path_layer,
            point: champlain_point,
        }
    }
}
