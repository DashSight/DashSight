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
    // Create the IIO context
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

    // Create the IMU accel device
    let imu_name = "lsm9ds1-imu_accel";
    let imu_accel_dev = ctx.find_device(imu_name).unwrap_or_else(|| {
        println!("Error opening device: {}", imu_name);
        process::exit(1);
    });

    // Create the IMU gyro device
    let imu_name = "lsm9ds1-imu_gyro";
    let imu_gyro_dev = ctx.find_device(imu_name).unwrap_or_else(|| {
        println!("Error opening device: {}", imu_name);
        process::exit(1);
    });

    // Get the IMU acceleration channels
    let x_accel_chan = imu_accel_dev
        .find_channel("accel_x", false)
        .unwrap_or_else(|| {
            println!("No 'accel_x' channel on this device");
            process::exit(1);
        });
    let y_accel_chan = imu_accel_dev
        .find_channel("accel_y", false)
        .unwrap_or_else(|| {
            println!("No 'accel_y' channel on this device");
            process::exit(1);
        });
    let z_accel_chan = imu_accel_dev
        .find_channel("accel_z", false)
        .unwrap_or_else(|| {
            println!("No 'accel_z' channel on this device");
            process::exit(1);
        });

    // Get the IMU gyro channels
    let x_gyro_chan = imu_gyro_dev
        .find_channel("anglvel_x", false)
        .unwrap_or_else(|| {
            println!("No 'anglvel_x' channel on this device");
            process::exit(1);
        });
    let y_gyro_chan = imu_gyro_dev
        .find_channel("anglvel_y", false)
        .unwrap_or_else(|| {
            println!("No 'anglvel_y' channel on this device");
            process::exit(1);
        });
    let z_gyro_chan = imu_gyro_dev
        .find_channel("anglvel_z", false)
        .unwrap_or_else(|| {
            println!("No 'anglvel_z' channel on this device");
            process::exit(1);
        });

    let accel_chan: [iio::channel::Channel; 3] = [x_accel_chan, y_accel_chan, z_accel_chan];
    let gyro_chan: [iio::channel::Channel; 3] = [x_gyro_chan, y_gyro_chan, z_gyro_chan];
    let mut calib = [0.0, 0.0, 0.0];
    let mut accel_scale = [1.0, 1.0, 1.0];
    let mut gyro_scale = [1.0, 1.0, 1.0];

    // Get the acceleration calibration offset
    for (i, ac) in accel_chan.iter().enumerate() {
        if let Ok(val) = ac.attr_read_int("calibbias") {
            calib[i] = val as f64;
        }
    }

    // Get the acceleration scale
    for (i, ac) in accel_chan.iter().enumerate() {
        if let Ok(val) = ac.attr_read_float("scale") {
            accel_scale[i] = val;
        }
    }

    // Get the gyro scale
    for (i, ac) in gyro_chan.iter().enumerate() {
        if let Ok(val) = ac.attr_read_float("scale") {
            gyro_scale[i] = val;
        }
    }

    // Set the acceleration sampling frequency
    for ac in accel_chan.iter() {
        ac.attr_write_float("sampling_frequency", 238.0).unwrap();
    }

    // Open the file to save data
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

    // Write the CVS headers
    write!(fd, "accel x, accel y, accel z, gyro x, gyro y, gyro z\n").unwrap();

    while !thread_info.close.lock().unwrap().get() {
        let mut g = [0.0, 0.0, 0.0];
        for (i, ac) in accel_chan.iter().enumerate() {
            if let Ok(val) = ac.attr_read_int("raw") {
                g[i] = (val as f64 - calib[i]) * accel_scale[i];

                write!(fd, "{}", g[i]).unwrap();
            }
            write!(fd, ",").unwrap();
        }

        let mut rot = [0.0, 0.0, 0.0];
        for (i, gc) in gyro_chan.iter().enumerate() {
            if let Ok(val) = gc.attr_read_int("raw") {
                rot[i] = val as f64 * gyro_scale[i];

                write!(fd, "{}", rot[i]).unwrap();
            }
            if i < 2 {
                write!(fd, ",").unwrap();
            }
        }

        write!(fd, "\n").unwrap();

        // TODO: Convert this to x and y values based on orientation
        thread_info.imu_tx.send((g[0], g[1])).unwrap();
    }

    fd.flush().unwrap();
}
