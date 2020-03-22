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
