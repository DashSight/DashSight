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
use crate::drive::read_track::Coord;
use crate::drive::threading::ThreadingRef;
use crate::utils::{genereate_polygon, lat_lon_comp, right_direction};
use gpsd_proto::handshake;
use nalgebra::geometry::{Isometry2, Point2};
use ncollide2d::query::PointQuery;
use std::fs::OpenOptions;
use std::io;
use std::net::TcpStream;
use std::time::{Duration, SystemTime};
use std::vec::Vec;

pub fn gpsd_thread(
    thread_info: ThreadingRef,
    times_tx: std::sync::mpsc::Sender<(Duration, Duration, Duration)>,
    time_diff_tx: std::sync::mpsc::Sender<(bool, Duration)>,
    location_tx: std::sync::mpsc::Sender<(f64, f64, i32, Option<bool>)>,
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

    let mut lap_times: Vec<(Coord, Duration)> = Vec::new();

    while !thread_info.close.lock().unwrap().get() {
        let msg = crate::utils::get_gps_lat_lon(&mut reader);

        match msg {
            Ok((lat, lon, _alt, status, _time, _speed, track)) => {
                // Check to see if we should start the timer
                if !thread_info.on_track.lock().unwrap().get()
                    && start_poly.contains_point(&Isometry2::identity(), &Point2::new(lat, lon))
                    && right_direction(course_info.start.head, track)
                {
                    let mut lap_start = thread_info.lap_start.write().unwrap();
                    *lap_start = SystemTime::now();
                    thread_info.on_track.lock().unwrap().set(true);
                    thread_info.start_on_track.lock().unwrap().set(true);
                    lap_times.clear();
                }

                // Check to see if we should stop the timer
                if thread_info.on_track.lock().unwrap().get()
                    && finish_poly.contains_point(&Isometry2::identity(), &Point2::new(lat, lon))
                    && right_direction(course_info.finish.head, track)
                {
                    thread_info.on_track.lock().unwrap().set(false);

                    match thread_info.lap_start.read().unwrap().elapsed() {
                        Ok(elapsed) => {
                            course_info.times.push(elapsed);
                            course_info.last = elapsed;
                            course_info.times.sort_unstable();
                            if let Some(worst) = course_info.times.last() {
                                course_info.worst = *worst;
                            }
                            if let Some(best) = course_info.times.first() {
                                course_info.best = *best;
                                // If we just set the best time, update the
                                // best_times vector
                                if *best == elapsed {
                                    course_info.best_times.clear();
                                    course_info.best_times.append(&mut lap_times);
                                }
                            }
                            times_tx
                                .send((course_info.last, course_info.best, course_info.worst))
                                .unwrap();

                            let (_, el) = &course_info.best_times.last().unwrap();

                            // Update the diff display
                            if let Some(diff) = el.checked_sub(elapsed) {
                                time_diff_tx.send((true, diff)).unwrap();
                            }
                            // Check if elapsed - best is greater then 0
                            // In this case we are slower then previous best
                            if let Some(diff) = elapsed.checked_sub(*el) {
                                time_diff_tx.send((false, diff)).unwrap();
                            }
                        }
                        Err(e) => {
                            println!("Error: {:?}", e);
                        }
                    }
                }

                // Save lap time data
                let mut time_delta_diff: Option<bool> = None;
                if thread_info.on_track.lock().unwrap().get() {
                    // Save the current location and time to a vector
                    match thread_info.lap_start.read().unwrap().elapsed() {
                        Ok(elapsed) => {
                            lap_times.push((
                                Coord {
                                    lat,
                                    lon,
                                    head: None,
                                },
                                elapsed,
                            ));
                        }
                        Err(e) => {
                            println!("Error: {:?}", e);
                        }
                    }

                    for (loc, el) in &course_info.best_times {
                        if lat_lon_comp(loc.lat, loc.lon, lat, lon) {
                            // This point matches a previous point
                            course_info.last_location_time = Some(*el);
                        }
                    }

                    match thread_info.lap_start.read().unwrap().elapsed() {
                        Ok(elapsed) => {
                            match course_info.last_location_time {
                                Some(llt) => {
                                    // Check if best - elapsed is greater then 0
                                    // In this case we are quicker then previous best
                                    if let Some(diff) = llt.checked_sub(elapsed) {
                                        time_delta_diff = Some(true);
                                        time_diff_tx.send((true, diff)).unwrap();
                                    }
                                    // Check if elapsed - best is greater then 0
                                    // In this case we are slower then previous best
                                    if let Some(diff) = elapsed.checked_sub(llt) {
                                        time_delta_diff = Some(false);
                                        time_diff_tx.send((false, diff)).unwrap();
                                    }
                                }
                                None => {
                                    // No time data, just reset to +00:00:000
                                    time_delta_diff = None;
                                    time_diff_tx.send((false, Duration::new(0, 0))).unwrap();
                                }
                            }
                        }
                        Err(e) => {
                            println!("Error: {:?}", e);
                        }
                    }
                }

                location_tx
                    .send((lat, lon, status, time_delta_diff))
                    .unwrap();
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
