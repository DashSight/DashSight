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

    println!("Starting IMU thread");

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

    println!("Device: {}", dev.name().unwrap());

    for chan in dev.channels() {
        println!("  chan: {:?}", chan.id().unwrap());
        if chan.has_attr("calibbias") {
            println!("     -> {:>9}", chan.id().unwrap());
        }
    }

    println!();

    loop {
        for chan in dev.channels() {
            if let Ok(val) = chan.attr_read_int("calibbias") {
                println!(" {:>9} => {:>8} ", chan.id().unwrap(), val);
            }
        }
        println!();
    }
}
