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

pub fn lat_lon_comp(lat_1: f64, lon_1: f64, lat_2: f64, lon_2: f64) -> bool {
    let round_margin = 5000.0;

    let lat_1_round = (lat_1 * round_margin).round() / round_margin;
    let lon_1_round = (lon_1 * round_margin).round() / round_margin;

    let lat_2_round = (lat_2 * round_margin).round() / round_margin;
    let lon_2_round = (lon_2 * round_margin).round() / round_margin;

    lat_1_round == lat_2_round && lon_1_round == lon_2_round
}

pub fn get_gps_lat_lon(reader: &mut dyn io::BufRead) -> Result<(f64, f64, f32, String), ()> {
    loop {
        let msg = get_data(reader);
        let gpsd_message;

        match msg {
            Ok(msg) => {
                gpsd_message = msg;
            }
            Err(_err) => {
                return Err(());
            }
        }

        match gpsd_message {
            ResponseData::Device(_) => {}
            ResponseData::Tpv(t) => {
                match t.lat {
                    Some(lat) => {
                        return Ok((lat, t.lon.unwrap(), t.alt.unwrap(), t.time.unwrap()));
                    }
                    _ => {
                        return Err(());
                    }
                };
            }
            ResponseData::Sky(_) => {}
            ResponseData::Pps(_) => {}
            ResponseData::Gst(_) => {}
        }
    }
}

pub struct Kalman {
    last_lat: f64,
    last_lon: f64,
    last_time: u128,
    accuracy: f64,
    variance: Option<f64>,
    q: f64,
}

impl Kalman {
    pub fn new(accuracy: f64) -> Kalman {
        Kalman {
            last_lat: 0.0,
            last_lon: 0.0,
            last_time: 0,
            accuracy: accuracy,
            variance: None,
            q: 3.0,
        }
    }

    pub fn process(&mut self, lat: f64, lon: f64, time: u128) -> (f64, f64) {
        match self.variance {
            None => {
                self.last_time = time;
                self.last_lat = lat;
                self.last_lon = lon;
                self.variance = Some(self.accuracy * self.accuracy);
            }
            Some(mut variance) => {
                let time_diff = time - self.last_time;

                if time_diff > 0 {
                    variance = variance + (time_diff as f64 * self.q * self.q / 1000.0);
                    self.last_time = time;
                }

                let k: f64 = variance / (variance + self.accuracy * self.accuracy);

                self.variance = Some((1.0 - k) * variance);

                self.last_lat = self.last_lat + (k * (lat - self.last_lat));
                self.last_lon = self.last_lon + (k * (lon - self.last_lon));
            }
        }

        (self.last_lat, self.last_lon)
    }
}
