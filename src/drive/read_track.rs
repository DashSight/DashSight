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

use serde::{Deserialize, Serialize};
use std::io::BufRead;

#[derive(Serialize, Deserialize)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
    pub head: Option<f32>,
}

pub fn get_long_and_lat(
    reader: std::io::BufReader<std::fs::File>,
) -> Vec<crate::drive::read_track::Coord> {
    let mut reader_iterator = reader.lines().map(|l| l.unwrap());
    let mut coord_vec = Vec::new();

    // Skip the first 13 lines
    // TODO: Be better with verification
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();
    reader_iterator.next();

    let mut lat: f64 = 0.0;
    let mut lon: f64 = 0.0;
    let mut head: Option<f32> = None;

    for line in reader_iterator {
        let trim_line = line.trim();
        if let Some(trkpt_num) = trim_line.find("<trkpt") {
            if let Some(trkpt_line) = trim_line.get((trkpt_num + 5)..) {
                let split_line: Vec<&str> = trkpt_line.split('"').collect();

                lat = split_line[1].parse().unwrap();
                lon = split_line[3].parse().unwrap();
            }
        } else if let Some(degrees_num) = trim_line.find("<degreesType") {
            if let Some(degrees_line) = trim_line.get((degrees_num + 1)..) {
                let split_line: Vec<&str> = degrees_line.split('<').collect();

                head = Some(split_line[0].parse().unwrap());
            }
        } else if let Some(_) = trim_line.find("</trkpt") {
            // Let's assume a lat/lon of 0 is just invalid
            if lat != 0.0 && lon != 0.0 {
                let c = Coord { lat, lon, head };
                coord_vec.push(c);
            }
        }
    }

    coord_vec
}
