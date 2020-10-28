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
use ncollide2d::shape::ConvexPolygon;
use std::fs::OpenOptions;
use std::io;
use std::net::TcpStream;
use std::time::{Duration, SystemTime};
use std::vec::Vec;

pub fn gpsd_thread(
    thread_info: ThreadingRef,
    elapsed_tx: std::sync::mpsc::Sender<Duration>,
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
        course_info.segments.first().unwrap().start.lat,
        course_info.segments.first().unwrap().start.lon,
        course_info
            .segments
            .first()
            .unwrap()
            .start
            .head
            .unwrap_or(0.0),
    );
    let finish_poly = genereate_polygon(
        course_info.segments.last().unwrap().finish.lat,
        course_info.segments.last().unwrap().finish.lon,
        course_info
            .segments
            .last()
            .unwrap()
            .finish
            .head
            .unwrap_or(0.0),
    );

    let mut segment_times: Vec<(Coord, Duration)> = Vec::new();
    let mut lap_times: Vec<Vec<(Coord, Duration)>> = Vec::new();
    let mut segment_starts: Vec<ConvexPolygon<f64>> = Vec::new();

    for segment in &course_info.segments {
        segment_starts.push(genereate_polygon(
            segment.start.lat,
            segment.start.lon,
            segment.start.head.unwrap_or(0.0),
        ));
    }

    let mut lap_start = SystemTime::now();
    let mut elapsed_time: Duration = Duration::from_secs(0);
    let mut on_track: bool = false;

    while !thread_info.close.lock().unwrap().get() {
        let msg = crate::utils::get_gps_lat_lon(&mut reader);

        match msg {
            Ok((lat, lon, _alt, status, _time, _speed, track)) => {
                // Check to see if we should start the timer
                if !on_track
                    && start_poly.contains_point(&Isometry2::identity(), &Point2::new(lat, lon))
                    && right_direction(course_info.segments.first().unwrap().start.head, track)
                {
                    lap_start = SystemTime::now();
                    on_track = true;
                    thread_info.on_track.lock().unwrap().set(true);
                    thread_info.start_on_track.lock().unwrap().set(true);
                    lap_times.clear();
                }

                // Check to see if we should stop the timer
                if on_track
                    && finish_poly.contains_point(&Isometry2::identity(), &Point2::new(lat, lon))
                    && right_direction(course_info.segments.last().unwrap().finish.head, track)
                {
                    thread_info.on_track.lock().unwrap().set(false);
                    on_track = false;

                    course_info.times.push(elapsed_time);
                    course_info.last = elapsed_time;
                    course_info.times.sort_unstable();
                    if let Some(worst) = course_info.times.last() {
                        course_info.worst = *worst;
                    }
                    if let Some(best) = course_info.times.first() {
                        course_info.best = *best;
                        // If we just set the best time, update the
                        // best_times vector
                        if *best == elapsed_time {
                            course_info.best_times.clear();
                            course_info.best_times.append(&mut lap_times);
                        }
                    }
                    times_tx
                        .send((course_info.last, course_info.best, course_info.worst))
                        .unwrap();

                    // Update the diff display
                    if let Some(diff) = course_info.best.checked_sub(elapsed_time) {
                        time_diff_tx.send((true, diff)).unwrap();
                    }
                    // Check if elapsed_time - best is greater then 0
                    // In this case we are slower then previous best
                    if let Some(diff) = elapsed_time.checked_sub(course_info.best) {
                        time_diff_tx.send((false, diff)).unwrap();
                    }
                } else if on_track {
                    elapsed_time = lap_start.elapsed().unwrap();
                    elapsed_tx.send(elapsed_time).unwrap();
                }

                // Save lap time data
                let mut time_delta_diff: Option<bool> = None;
                if on_track {
                    // Save the current location and time to a vector
                    segment_times.push((
                        Coord {
                            lat,
                            lon,
                            head: None,
                        },
                        elapsed_time,
                    ));

                    // Split a new segment
                    for segment in &segment_starts {
                        // Check if we match the start of a segment
                        if segment.contains_point(&Isometry2::identity(), &Point2::new(lat, lon)) {
                            lap_times.push(segment_times);
                            segment_times = Vec::new();
                        }
                    }

                    // Check if the current location matches a previous one
                    for segment in &course_info.best_times {
                        for (loc, el) in segment {
                            if lat_lon_comp(loc.lat, loc.lon, lat, lon) {
                                // This point matches a previous point
                                course_info.last_location_time =
                                    Some(*el - segment.first().unwrap().1);
                            }
                        }
                    }

                    let segment_diff = elapsed_time - lap_times.last().unwrap().first().unwrap().1;

                    match course_info.last_location_time {
                        Some(llt) => {
                            // Check if best - segment_diff is greater then 0
                            // In this case we are quicker then previous best
                            if let Some(diff) = llt.checked_sub(segment_diff) {
                                time_delta_diff = Some(true);
                                time_diff_tx.send((true, diff)).unwrap();
                            }
                            // Check if segment_diff - best is greater then 0
                            // In this case we are slower then previous best
                            if let Some(diff) = segment_diff.checked_sub(llt) {
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
