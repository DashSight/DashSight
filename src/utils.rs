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
use gpsd_proto::{get_data, ResponseData};
use std::io;

#[macro_export]
macro_rules! upgrade_weak {
    ($x:expr, $r:expr) => {{
        match $x.upgrade() {
            Some(o) => o,
            None => return $r,
        }
    }};
    ($x:expr) => {
        upgrade_weak!($x, ())
    };
}

/// Generate a polygon based on the information provided.
/// This takes the start or end latitude, longitude and heading
/// to generate a polygon we use to determine if we have crossed
/// the start or end.
/// We ignore the curvature of the Earth to simplify this.
pub fn genereate_polygon(lat: f64, lon: f64, track: f32) -> (Coord, Coord, Coord, Coord) {
    // Be lazy and assume
    //    - 111,111 metres is 1 degree latitude
    //    - 111,111 * cos(lat) meters is 1 degree longitude

    let top_left_x = (2.0 * (track * std::f32::consts::PI / 180.0).cos())
        - (0.5 * (track * std::f32::consts::PI / 180.0).sin());
    let top_left_y = (2.0 * (track * std::f32::consts::PI / 180.0).sin())
        + (0.5 * (track * std::f32::consts::PI / 180.0).cos());

    let bot_left_x = (2.0 * (track * std::f32::consts::PI / 180.0).cos())
        + (0.5 * (track * std::f32::consts::PI / 180.0).sin());
    let bot_left_y = (2.0 * (track * std::f32::consts::PI / 180.0).sin())
        - (0.5 * (track * std::f32::consts::PI / 180.0).cos());

    let top_left = Coord {
        lat: lat + (top_left_y as f64 / 111111.0),
        lon: lon - (top_left_x as f64 / (111111.0 * lat.cos())),
        head: None,
    };
    let bot_left = Coord {
        lat: lat + (bot_left_y as f64 / 111111.0),
        lon: lon - (bot_left_x as f64 / (111111.0 * lat.cos())),
        head: None,
    };
    let top_right = Coord {
        lat: lat - (bot_left_y as f64 / 111111.0),
        lon: lon + (bot_left_x as f64 / (111111.0 * lat.cos())),
        head: None,
    };
    let bot_right = Coord {
        lat: lat - (top_left_y as f64 / 111111.0),
        lon: lon + (top_left_x as f64 / (111111.0 * lat.cos())),
        head: None,
    };

    (top_left, bot_left, top_right, bot_right)
}

pub fn lat_lon_comp(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> bool {
    let earth = 6378.137; // Radius of earth in km
    let error = 1.0; // Error range, in metres

    let d_lat = (lat2 * std::f64::consts::PI / 180.0) - (lat1 * std::f64::consts::PI / 180.0);
    let d_lon = (lon2 * std::f64::consts::PI / 180.0) - (lon1 * std::f64::consts::PI / 180.0);

    let a = ((d_lat / 2.0).sin()).powi(2)
        + (lat1 * std::f64::consts::PI / 180.0).cos()
            * (lat2 * std::f64::consts::PI / 180.0).cos()
            * ((d_lon / 2.0).sin()).powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    let d = earth * c * 1000.0;

    d < error
}

/// Gets the relevent location/velocity data from the GPS device
/// Returns latitude, longitude, altitude, time, speed and track
pub fn get_gps_lat_lon(
    reader: &mut dyn io::BufRead,
) -> Result<(f64, f64, f32, String, f32, f32), ()> {
    loop {
        let msg = get_data(reader);
        let gpsd_message = match msg {
            Ok(msg) => msg,
            Err(_err) => {
                return Err(());
            }
        };

        println!("gpsd_message: {:?}", gpsd_message);

        match gpsd_message {
            ResponseData::Device(_) => {}
            ResponseData::Tpv(t) => {
                // Check if we have a longitude and latitude
                if t.lat.is_some() && t.lon.is_some() {
                    // Return the longitude and latitude
                    // If we don't have a time (which apparently can happen)
                    // then return the Unix Epoch start time instead
                    return Ok((
                        t.lat.unwrap(),
                        t.lon.unwrap(),
                        t.alt.unwrap(),
                        t.time.unwrap_or("1970-01-01T00:00:00.000Z".to_string()),
                        t.speed.unwrap(),
                        t.track.unwrap(),
                    ));
                }
            }
            ResponseData::Sky(_) => {}
            ResponseData::Pps(_) => {}
            ResponseData::Gst(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison() {
        assert_eq!(lat_lon_comp(10.0, 10.0, 10.0, 10.0), true);
        assert_eq!(lat_lon_comp(10.0, 10.0, 15.0, 15.0), false);
    }

    #[test]
    fn test_poly() {
        let top_left = Coord {
            lat: 37.32449490991848,
            lon: -121.92461158738175,
            head: None,
        };
        let bot_left = Coord {
            lat: 37.32448854595066,
            lon: -121.92461842563702,
            head: None,
        };
        let top_right = Coord {
            lat: 37.324469454049336,
            lon: -121.92458423436298,
            head: None,
        };
        let bot_right = Coord {
            lat: 37.324463090081515,
            lon: -121.92459107261826,
            head: None,
        };

        let poly = genereate_polygon(37.32447900, -121.92460133, 45.0);

        assert_eq!(poly.0.lat, top_left.lat);
        assert_eq!(poly.0.lon, top_left.lon);

        assert_eq!(poly.1.lat, bot_left.lat);
        assert_eq!(poly.1.lon, bot_left.lon);

        assert_eq!(poly.2.lat, top_right.lat);
        assert_eq!(poly.2.lon, top_right.lon);

        assert_eq!(poly.3.lat, bot_right.lat);
        assert_eq!(poly.3.lon, bot_right.lon);
    }
}
