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
use std::process;

pub fn imu_thread(_thread_info: ThreadingRef) {
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

    loop {
        if let Ok(val) = x_chan.attr_read_int("calibbias") {
            println!(" {:>9} => {:>8} ", x_chan.id().unwrap(), val);
        }
        if let Ok(val) = y_chan.attr_read_int("calibbias") {
            println!(" {:>9} => {:>8} ", y_chan.id().unwrap(), val);
        }
        if let Ok(val) = z_chan.attr_read_int("calibbias") {
            println!(" {:>9} => {:>8} ", z_chan.id().unwrap(), val);
        }
    }
}
