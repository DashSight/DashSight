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

use crate::drive::course::Course;
use crate::drive::threading::ThreadingRef;
use crate::utils::{genereate_polygon, right_direction};
use gpsd_proto::handshake;
use nalgebra::geometry::{Isometry2, Point2};
use ncollide2d::query::point_internal::point_query::PointQuery;
use std::fs::OpenOptions;
use std::io;
use std::net::TcpStream;
use std::time::{Duration, SystemTime};

pub fn gpsd_thread(
    thread_info: ThreadingRef,
    times_tx: std::sync::mpsc::Sender<(Duration, Duration, Duration)>,
    location_tx: std::sync::mpsc::Sender<(f64, f64)>,
    course_info: &mut Course,
) {
    let gpsd_connect;

    loop {
        let stream = TcpStream::connect("127.0.0.1:2947");
        match stream {
            Ok(stream) => {
                gpsd_connect = stream;
                break;
            }
            Err(err) => {
                println!("Failed to connect to GPSD: {:?}", err);
                if thread_info.close.lock().unwrap().get() {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
        }
    }

    let mut reader = io::BufReader::new(&gpsd_connect);
    let mut writer = io::BufWriter::new(&gpsd_connect);

    handshake(&mut reader, &mut writer).unwrap();

    println!(
        "Creating start Poly from {}, {}, heading: {}",
        course_info.start.lat,
        course_info.start.lon,
        course_info.start.head.unwrap_or(0.0)
    );
    let start_poly = genereate_polygon(
        course_info.start.lat,
        course_info.start.lon,
        course_info.start.head.unwrap_or(0.0),
    );
    let finish_poly = genereate_polygon(
        course_info.finish.lat,
        course_info.finish.lon,
        course_info.start.head.unwrap_or(0.0),
    );

    while !thread_info.close.lock().unwrap().get() {
        let msg = crate::utils::get_gps_lat_lon(&mut reader);

        match msg {
            Ok((lat, lon, _alt, _time, _speed, track)) => {
                location_tx.send((lat, lon)).unwrap();

                if !thread_info.on_track.lock().unwrap().get()
                    && start_poly.contains_point(&Isometry2::identity(), &Point2::new(lat, lon))
                    && right_direction(course_info.start.head, track)
                {
                    let mut lap_start = thread_info.lap_start.write().unwrap();
                    *lap_start = SystemTime::now();
                    thread_info.on_track.lock().unwrap().set(true);
                    thread_info.change_colour.lock().unwrap().set(true);
                } else {
                    println!("Point {}, {} is not inside", lat, lon);
                }

                if thread_info.on_track.lock().unwrap().get()
                    && finish_poly.contains_point(&Isometry2::identity(), &Point2::new(lat, lon))
                    && right_direction(course_info.finish.head, track)
                {
                    thread_info.on_track.lock().unwrap().set(false);
                    thread_info.change_colour.lock().unwrap().set(true);

                    match thread_info.lap_start.read().unwrap().elapsed() {
                        Ok(elapsed) => {
                            course_info.times.push(elapsed);
                            course_info.last = elapsed;
                            course_info.times.sort_unstable();
                            if let Some(worst) = course_info.times.last() {
                                course_info.worst = *worst
                            }
                            if let Some(best) = course_info.times.first() {
                                course_info.best = *best
                            }
                            times_tx
                                .send((course_info.last, course_info.best, course_info.worst))
                                .unwrap();
                        }
                        Err(e) => {
                            println!("Error: {:?}", e);
                        }
                    }
                }
            }
            Err(err) => {
                println!("Failed to get a message from GPSD: {:?}", err);
                std::thread::sleep(std::time::Duration::from_millis(30));
                continue;
            }
        }

        if thread_info.serialise.lock().unwrap().get() {
            let mut track_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(thread_info.time_file.read().unwrap().clone());

            match track_file.as_mut() {
                Ok(fd) => {
                    let serialized = serde_json::to_string(&course_info).unwrap();

                    serde_json::to_writer(fd, &serialized).unwrap();
                }
                Err(e) => {
                    println!("Unable to open file: {:?}", e);
                }
            }
        }
    }
}
