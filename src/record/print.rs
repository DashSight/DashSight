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

use std::fs::File;
use std::io::Write;

pub fn gpx_start(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"<?xml version=\"1.0\" encoding=\"utf-8\"?>\n")?;
    fd.write_all(b"<gpx version=\"1.1\" creator=\"DashSight\"\n")?;
    fd.write_all(b"        xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n")?;
    fd.write_all(b"        xmlns=\"http://www.topografix.com/GPX/1.1\"\n")?;
    fd.write_all(b"        xsi:schemaLocation=\"http://www.topografix.com/GPS/1/1\n")?;
    fd.write_all(b"        http://www.topografix.com/GPX/1/1/gpx.xsd\">\n")?;
    Ok(())
}

pub fn gpx_stop(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"</gpx>\n")?;
    Ok(())
}

pub fn gpx_metadata(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"  <metadata>\n")?;
    fd.write_all(b"    <link href=\"https://github.com/DashSight/DashSight\">\n")?;
    fd.write_all(b"      <text>DashSight</text>\n")?;
    fd.write_all(b"    </link>\n")?;
    fd.write_all(b"  </metadata>\n")?;
    Ok(())
}

pub fn gpx_track_start(fd: &mut File, track_name: String) -> Result<(), std::io::Error> {
    fd.write_all(b"  <trk>\n")?;
    writeln!(fd, "    <name>{}</name>", track_name)?;
    Ok(())
}

pub fn gpx_track_stop(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"  </trk>\n")?;
    Ok(())
}

pub fn gpx_point_info(
    fd: &mut File,
    lat: f64,
    lon: f64,
    alt: f32,
    time: String,
    heading: f32,
) -> Result<(), std::io::Error> {
    writeln!(fd, "      <trkpt lat=\"{}\" lon=\"{}\">", lat, lon)?;
    writeln!(fd, "        <ele>{}</ele>", alt)?;
    writeln!(fd, "        <time>{}</time>", time)?;
    writeln!(fd, "        <degreesType>{}</degreesType>", heading)?;
    writeln!(fd, "      </trkpt>")?;
    Ok(())
}

pub fn gpx_track_seg_start(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"    <trkseg>\n")?;
    Ok(())
}

pub fn gpx_track_seg_stop(fd: &mut File) -> Result<(), std::io::Error> {
    fd.write_all(b"    </trkseg>\n")?;
    Ok(())
}
