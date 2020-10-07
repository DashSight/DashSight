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
use ncollide2d::shape::ConvexPolygon;
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

/// Generate a polygon based on the information provided.
/// This takes the start or end latitude, longitude and heading
/// to generate a polygon we use to determine if we have crossed
/// the start or end.
/// We ignore the curvature of the Earth to simplify this.
pub fn genereate_polygon(lat: f64, lon: f64, track: f32) -> ConvexPolygon<f64> {
    // Be lazy and assume
    //    - 111,111 metres is 1 degree latitude
    //    - 111,111 * cos(lat) meters is 1 degree longitude

    let top_left_x = (5.0 * (track * std::f32::consts::PI / 180.0).cos())
        - (1.0 * (track * std::f32::consts::PI / 180.0).sin());
    let top_left_y = (5.0 * (track * std::f32::consts::PI / 180.0).sin())
        + (1.0 * (track * std::f32::consts::PI / 180.0).cos());

    let bot_left_x = (5.0 * (track * std::f32::consts::PI / 180.0).cos())
        + (1.0 * (track * std::f32::consts::PI / 180.0).sin());
    let bot_left_y = (5.0 * (track * std::f32::consts::PI / 180.0).sin())
        - (1.0 * (track * std::f32::consts::PI / 180.0).cos());

    let top_right_x = (5.0 * (track * std::f32::consts::PI / 180.0).cos())
        + (1.0 * (track * std::f32::consts::PI / 180.0).sin());
    let top_right_y = (5.0 * (track * std::f32::consts::PI / 180.0).sin())
        - (1.0 * (track * std::f32::consts::PI / 180.0).cos());

    let bot_right_x = (5.0 * (track * std::f32::consts::PI / 180.0).cos())
        - (1.0 * (track * std::f32::consts::PI / 180.0).sin());
    let bot_right_y = (5.0 * (track * std::f32::consts::PI / 180.0).sin())
        + (1.0 * (track * std::f32::consts::PI / 180.0).cos());

    let top_left = nalgebra::geometry::Point2::new(
        lat + (top_left_y as f64 / 111111.0),
        lon - (top_left_x as f64 / (111111.5 * lat.cos())),
    );
    let bot_left = nalgebra::geometry::Point2::new(
        lat + (bot_left_y as f64 / 111111.0),
        lon - (bot_left_x as f64 / (111111.5 * lat.cos())),
    );
    let top_right = nalgebra::geometry::Point2::new(
        lat - (top_right_y as f64 / 111111.0),
        lon + (top_right_x as f64 / (111111.5 * lat.cos())),
    );
    let bot_right = nalgebra::geometry::Point2::new(
        lat - (bot_right_y as f64 / 111111.0),
        lon + (bot_right_x as f64 / (111111.5 * lat.cos())),
    );

    let poly_points = vec![top_left, bot_left, bot_right, top_right];
    ConvexPolygon::try_new(poly_points).unwrap()
}

pub fn right_direction(recorded_heading: Option<f32>, current_heading: f32) -> bool {
    match recorded_heading {
        Some(rec) => {
            if current_heading == 0.0 {
                true
            } else {
                // Check overflow
                if rec < 30.0 {
                    if current_heading >= 0.0 && current_heading < rec + 30.0
                        || current_heading < 360.0 && current_heading >= 330.0 + rec
                    {
                        return true;
                    }

                    return false;
                }

                if rec > 330.0 {
                    if current_heading <= 360.0 && current_heading > rec - 30.0
                        || current_heading > 0.0 && current_heading <= rec - 330.0
                    {
                        return true;
                    }

                    return false;
                }

                if current_heading >= rec - 30.0 && current_heading <= rec + 30.0 {
                    return true;
                }

                false
            }
        }
        None => true,
    }
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

        match gpsd_message {
            ResponseData::Device(_) => {}
            ResponseData::Tpv(t) => {
                // Check if we have a longitude and latitude
                if t.lat.is_some() && t.lon.is_some() && t.alt.is_some() {
                    // Return the longitude and latitude
                    // If we don't have a time (which apparently can happen)
                    // then return the Unix Epoch start time instead
                    return Ok((
                        t.lat.unwrap(),
                        t.lon.unwrap(),
                        t.alt.unwrap(),
                        t.time
                            .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string()),
                        t.speed.unwrap_or(0.0),
                        t.track.unwrap_or(0.0),
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
    use ncollide2d::query::point_internal::point_query::PointQuery;

    #[test]
    fn test_poly() {
        let poly = genereate_polygon(37.32447900, -121.92460133, 45.0);

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

    #[test]
    fn test_poly2() {
        let poly = genereate_polygon(37.3244322, -121.9245186, 109.1828);

        assert_eq!(
            poly.contains_point(
                &nalgebra::geometry::Isometry2::identity(),
                &nalgebra::geometry::Point2::new(37.3244386, -121.9245781)
            ),
            false
        );

        assert_eq!(
            poly.contains_point(
                &nalgebra::geometry::Isometry2::identity(),
                &nalgebra::geometry::Point2::new(37.324432, -121.924509)
            ),
            true
        );
    }

    #[test]
    fn test_current_heading() {
        // We don't have enough information, just return true
        assert_eq!(right_direction(None, 3.14), true);
        assert_eq!(right_direction(Some(2.79), 0.0), true);

        assert_eq!(right_direction(Some(15.0), 350.0), true);
        assert_eq!(right_direction(Some(45.0), 45.0), true);
        assert_eq!(right_direction(Some(45.0), 75.0), true);
        assert_eq!(right_direction(Some(110.0), 130.0), true);
        assert_eq!(right_direction(Some(350.0), 10.0), true);
        assert_eq!(right_direction(Some(350.0), 20.0), true);

        assert_eq!(right_direction(Some(15.0), 340.0), false);
        assert_eq!(right_direction(Some(45.0), 1.0), false);
        assert_eq!(right_direction(Some(110.0), 79.0), false);
        assert_eq!(right_direction(Some(350.0), 21.0), false);
    }
}
