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
use ncollide2d::shape::Polyline;
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
pub fn genereate_polygon(lat: f64, lon: f64, track: f32) -> Polyline<f64> {
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

    let top_left = nalgebra::geometry::Point2::new(
        lat + (top_left_y as f64 / 111111.0),
        lon - (top_left_x as f64 / (111111.0 * lat.cos())),
    );
    let bot_left = nalgebra::geometry::Point2::new(
        lat + (bot_left_y as f64 / 111111.0),
        lon - (bot_left_x as f64 / (111111.0 * lat.cos())),
    );
    let top_right = nalgebra::geometry::Point2::new(
        lat - (bot_left_y as f64 / 111111.0),
        lon + (bot_left_x as f64 / (111111.0 * lat.cos())),
    );
    let bot_right = nalgebra::geometry::Point2::new(
        lat - (top_left_y as f64 / 111111.0),
        lon + (top_left_x as f64 / (111111.0 * lat.cos())),
    );

    let poly_points = vec![top_left, bot_left, top_right, bot_right];
    Polyline::new(poly_points, None)
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
    use crate::drive::read_track::Coord;
    use ncollide2d::query::point_internal::point_query::PointQuery;

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

        assert_eq!(poly.points()[0][0], top_left.lat);
        assert_eq!(poly.points()[0][1], top_left.lon);

        assert_eq!(poly.points()[1][0], bot_left.lat);
        assert_eq!(poly.points()[1][1], bot_left.lon);

        assert_eq!(poly.points()[2][0], top_right.lat);
        assert_eq!(poly.points()[2][1], top_right.lon);

        assert_eq!(poly.points()[3][0], bot_right.lat);
        assert_eq!(poly.points()[3][1], bot_right.lon);

        assert_eq!(
            poly.contains_point(
                &nalgebra::geometry::Isometry2::identity(),
                &nalgebra::geometry::Point2::new(37.32447900, -121.92460133)
            ),
            true
        );
        assert_eq!(
            poly.contains_point(
                &nalgebra::geometry::Isometry2::identity(),
                &nalgebra::geometry::Point2::new(37.02447900, -121.92460133)
            ),
            false
        );
        assert_eq!(
            poly.contains_point(
                &nalgebra::geometry::Isometry2::identity(),
                &nalgebra::geometry::Point2::new(37.32447900, -121.52460133)
            ),
            false
        );
    }
}
