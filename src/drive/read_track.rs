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

impl Coord {
    pub fn new(lat: f64, lon: f64, head: Option<f32>) -> Self {
        Self { lat, lon, head }
    }
}

pub fn get_long_and_lat(
    reader: std::io::BufReader<std::fs::File>,
) -> Vec<Vec<crate::drive::read_track::Coord>> {
    let mut reader_iterator = reader.lines().map(|l| l.unwrap());
    let mut track_vec = Vec::new();
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
                let split_line: Vec<&str> = split_line[0].split('>').collect();

                head = Some(split_line[1].parse().unwrap());
            }
        } else if trim_line.find("</trkpt").is_some() {
            // Let's assume a lat/lon of 0 is just invalid
            if lat != 0.0 && lon != 0.0 && head.unwrap_or(0.0) > 0.0 {
                let c = Coord { lat, lon, head };
                coord_vec.push(c);
            }
            lat = 0.0;
            lon = 0.0;
            head = None;
        } else if trim_line.find("</trkseg").is_some() {
            track_vec.push(coord_vec);
            coord_vec = Vec::new()
        }
    }

    // This is to maintain backwards compatability with single
    // segment tracks.
    if !coord_vec.is_empty() {
        track_vec.push(coord_vec);
    }

    track_vec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::genereate_polygon;
    use ncollide2d::query::PointQuery;
    use std::fs::OpenOptions;
    use std::io::BufReader;

    #[test]
    fn test_file_open() {
        let track_file = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open("tests/test-track-carpark");
        let reader = BufReader::new(track_file.unwrap());
        let _track_points = get_long_and_lat(reader);

        let track_file = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open("tests/test-track-cowpalace");
        let reader = BufReader::new(track_file.unwrap());
        let _track_points = get_long_and_lat(reader);

        let track_file = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open("tests/test-track-backyard");
        let reader = BufReader::new(track_file.unwrap());
        let _track_points = get_long_and_lat(reader);
    }

    #[test]
    fn test_file_poly_create() {
        let track_file = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open("tests/test-track-cowpalace");
        let reader = BufReader::new(track_file.unwrap());
        let track_points = get_long_and_lat(reader);

        let start_poly = genereate_polygon(
            track_points.first().unwrap().first().unwrap().lat,
            track_points.first().unwrap().first().unwrap().lon,
            track_points
                .first()
                .unwrap()
                .first()
                .unwrap()
                .head
                .unwrap_or(0.0),
        );

        assert_eq!(
            start_poly.contains_point(
                &nalgebra::geometry::Isometry2::identity(),
                &nalgebra::geometry::Point2::new(37.7060849, -122.4209836)
            ),
            true
        );

        assert_eq!(
            start_poly.contains_point(
                &nalgebra::geometry::Isometry2::identity(),
                &nalgebra::geometry::Point2::new(37.706084, -122.420983)
            ),
            true
        );

        assert_eq!(
            start_poly.contains_point(
                &nalgebra::geometry::Isometry2::identity(),
                &nalgebra::geometry::Point2::new(37.7060437, -122.4209761)
            ),
            false
        );
    }
}
