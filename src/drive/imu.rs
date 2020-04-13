/*
 * Copyright 2020 Alistair Francis <alistair@alistair23.me>
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

use crate::drive::drive::*;
use industrial_io as iio;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process;

pub fn imu_thread(thread_info: ThreadingRef, file_name: &mut PathBuf) {
    let ctx;

    match iio::Context::new() {
        Ok(c) => {
            ctx = c;
        }
        Err(e) => {
            println!("Error creating IIO context: {:?}", e);
            return;
        }
    }

    let dev = ctx.get_device(0).unwrap_or_else(|err| {
        println!("Error opening first device: {}", err);
        process::exit(1);
    });

    let x_chan = dev.find_channel("accel_x", false).unwrap_or_else(|| {
        println!("No 'accel_x' channel on this device");
        process::exit(1);
    });
    let y_chan = dev.find_channel("accel_y", false).unwrap_or_else(|| {
        println!("No 'accel_y' channel on this device");
        process::exit(1);
    });
    let z_chan = dev.find_channel("accel_z", false).unwrap_or_else(|| {
        println!("No 'accel_z' channel on this device");
        process::exit(1);
    });

    let mut x_calib = 0;
    let mut y_calib = 0;
    let mut z_calib = 0;

    if let Ok(val) = x_chan.attr_read_int("calibbias") {
        x_calib = val;
    }
    if let Ok(val) = y_chan.attr_read_int("calibbias") {
        y_calib = val;
    }
    if let Ok(val) = z_chan.attr_read_int("calibbias") {
        z_calib = val;
    }

    let mut name = file_name.file_stem().unwrap().to_str().unwrap().to_string();

    name.push_str("-imu.cvs");

    file_name.pop();
    file_name.push(name);

    let mut imu_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&file_name);

    let fd = imu_file.as_mut().unwrap();

    write!(fd, "x,y,z\n").unwrap();

    while !thread_info.close.lock().unwrap().get() {
        if let Ok(mut val) = x_chan.attr_read_int("raw") {
            val = val - x_calib;
            write!(fd, "{},", val).unwrap();
            println!(" {:>9} => {:>8} ", x_chan.id().unwrap(), val);
        }
        if let Ok(mut val) = y_chan.attr_read_int("raw") {
            val = val - y_calib;
            write!(fd, "{},", val).unwrap();
            println!(" {:>9} => {:>8} ", y_chan.id().unwrap(), val);
        }
        if let Ok(mut val) = z_chan.attr_read_int("raw") {
            val = val - z_calib;
            write!(fd, "{}\n", val).unwrap();
            println!(" {:>9} => {:>8} ", z_chan.id().unwrap(), val);
        }
    }

    fd.flush().unwrap();
}
