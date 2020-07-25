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

pub fn get_gps_lat_lon(reader: &mut dyn io::BufRead) -> Result<(f64, f64, f32, String, f32), ()> {
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
}
