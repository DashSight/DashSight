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

use crate::drive::threading::ThreadingRef;
use industrial_io as iio;
use std::path::PathBuf;
use std::process;

struct TempContext {
    infra_chan: Vec<iio::channel::Channel>,

    infra_offset: Vec<f64>,

    infra_scale: Vec<f64>,
}

impl TempContext {
    fn new(ctx: &iio::Context) -> TempContext {
        let mut infra_dev: Vec<iio::device::Device> = Vec::new();

        let dev_name = "mlx90614";
        for dev in ctx.devices() {
            if let Some(name) = dev.name() {
                if name == dev_name {
                    infra_dev.push(dev)
                }
            }
        }

        let mut infra_chan: Vec<iio::channel::Channel> = Vec::new();
        for dev in infra_dev {
            infra_chan.push(dev.find_channel("temp_object", false).unwrap_or_else(|| {
                println!("No 'temp_object' channel on this device");
                process::exit(1);
            }));
        }

        let mut infra_offset: Vec<f64> = Vec::new();
        for ic in infra_chan.iter() {
            if let Ok(val) = ic.attr_read_float("offset") {
                infra_offset.push(val);
            }
        }

        let mut infra_scale: Vec<f64> = Vec::new();
        for ic in infra_chan.iter() {
            if let Ok(val) = ic.attr_read_float("scale") {
                infra_scale.push(val);
            }
        }

        TempContext {
            infra_chan,
            infra_offset,
            infra_scale,
        }
    }

    fn num_temp_sensors(&self) -> usize {
        self.infra_chan.len()
    }

    fn get_temperature_celsius(&self) -> Vec<f64> {
        let mut infra_values: Vec<f64> = Vec::new();
        infra_values.resize(self.infra_chan.len(), 0.0);

        for (i, chan) in self.infra_chan.iter().enumerate() {
            if let Ok(val) = chan.attr_read_float("raw") {
                infra_values[i] = (val + self.infra_offset[i]) * self.infra_scale[i] / 1000.0;
            }
        }

        infra_values
    }
}

pub fn temp_thread(thread_info: ThreadingRef, _file_name: &mut PathBuf) {
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

    let temp_context = TempContext::new(&ctx);

    thread_info
        .temp_sensors
        .set(temp_context.num_temp_sensors());

    while !thread_info.close.lock().unwrap().get() {
        let temp = temp_context.get_temperature_celsius();

        thread_info.temp_tx.send(temp).unwrap();
    }
}
