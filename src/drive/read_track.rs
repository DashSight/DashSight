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

use std::io::BufRead;

pub struct Coord {
    lat: f64,
    lon: f64,
}

pub fn get_long_and_lat(
    reader: std::io::BufReader<std::fs::File>,
) -> Vec<crate::drive::read_track::Coord> {
    let mut reader_iterator = reader.lines().map(|l| l.unwrap());
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

    // for line in reader_iterator {
    //     if let Some(trkpt_num) = line.trim().find("trkpt") {
    //         line.trim().chars().skip(trkpt_num) {
    //             let split_line = trkpt_line.split('"');
    //         }
    //     }
    // }

    coord_vec
}

// pub fn gpx_stop(fd: &mut File) -> Result<(), std::io::Error> {
//     fd.write_all(b"</gpx>\n")?;
//     Ok(())
// }

// pub fn gpx_metadata(fd: &mut File) -> Result<(), std::io::Error> {
//     fd.write_all(b"  <metadata>\n")?;
//     fd.write_all(b"    <link href=\"https://github.com/alistair23/DashSight\">\n")?;
//     fd.write_all(b"      <text>DashSight</text>\n")?;
//     fd.write_all(b"    </link>\n")?;
//     fd.write_all(b"  </metadata>\n")?;
//     Ok(())
// }

// pub fn gpx_track_start(fd: &mut File, track_name: String) -> Result<(), std::io::Error> {
//     fd.write_all(b"  <trk>\n")?;
//     write!(fd, "    <name>{}</name>\n", track_name)?;
//     Ok(())
// }

// pub fn gpx_track_stop(fd: &mut File) -> Result<(), std::io::Error> {
//     fd.write_all(b"    </trkseg>\n")?;
//     fd.write_all(b"  </trk>\n")?;
//     Ok(())
// }

// pub fn gpx_point_info(
//     fd: &mut File,
//     lat: f64,
//     lon: f64,
//     alt: f32,
//     time: String,
// ) -> Result<(), std::io::Error> {
//     write!(fd, "      <trkpt lat=\"{}\" lon=\"{}\">\n", lat, lon)?;
//     write!(fd, "        <ele>{}git f</ele>\n", alt)?;
//     write!(fd, "        <time>{}</time>\n", time)?;
//     write!(fd, "      </trkpt>\n")?;
//     Ok(())
// }

// pub fn gpx_track_seg_start(fd: &mut File) -> Result<(), std::io::Error> {
//     fd.write_all(b"    <trkseg>\n")?;
//     Ok(())
// }

// pub fn gpx_track_seg_stop(fd: &mut File) -> Result<(), std::io::Error> {
//     fd.write_all(b"    </trkseg>\n")?;
//     Ok(())
// }
