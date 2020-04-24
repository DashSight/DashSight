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
use ahrs::{Ahrs, Madgwick};
use industrial_io as iio;
use nalgebra::geometry::UnitQuaternion;
use nalgebra::Vector3;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process;

struct ImuContext {
    accel_chan: [iio::channel::Channel; 3],
    gyro_chan: [iio::channel::Channel; 3],
    mag_chan: [iio::channel::Channel; 3],

    accel_calib: [f64; 3],

    accel_scale: [f64; 3],
    gyro_scale: [f64; 3],
    mag_scale: [f64; 3],
}

impl ImuContext {
    fn new(ctx: &iio::Context) -> ImuContext {
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

        // Create the IMU mag device
        let imu_name = "lsm9ds1_magn";
        let imu_mag_dev = ctx.find_device(imu_name).unwrap_or_else(|| {
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

        // Get the IMU mag channels
        let x_mag_chan = imu_mag_dev
            .find_channel("magn_x", false)
            .unwrap_or_else(|| {
                println!("No 'magn_x' channel on this device");
                process::exit(1);
            });
        let y_mag_chan = imu_mag_dev
            .find_channel("magn_y", false)
            .unwrap_or_else(|| {
                println!("No 'magn_y' channel on this device");
                process::exit(1);
            });
        let z_mag_chan = imu_mag_dev
            .find_channel("magn_z", false)
            .unwrap_or_else(|| {
                println!("No 'magn_z' channel on this device");
                process::exit(1);
            });

        let accel_chan: [iio::channel::Channel; 3] = [x_accel_chan, y_accel_chan, z_accel_chan];
        let gyro_chan: [iio::channel::Channel; 3] = [x_gyro_chan, y_gyro_chan, z_gyro_chan];
        let mag_chan: [iio::channel::Channel; 3] = [x_mag_chan, y_mag_chan, z_mag_chan];

        ImuContext {
            accel_chan,
            gyro_chan,
            mag_chan,
            accel_calib: [0.0, 0.0, 0.0],
            accel_scale: [0.0, 0.0, 0.0],
            gyro_scale: [0.0, 0.0, 0.0],
            mag_scale: [0.0, 0.0, 0.0],
        }
    }

    fn get_calibration_data(&mut self) {
        // Get the acceleration calibration offset
        for (i, ac) in self.accel_chan.iter().enumerate() {
            if let Ok(val) = ac.attr_read_int("calibbias") {
                self.accel_calib[i] = val as f64;
            }
        }
    }

    fn get_scale_data(&mut self) {
        // Get the acceleration scale
        for (i, ac) in self.accel_chan.iter().enumerate() {
            if let Ok(val) = ac.attr_read_float("scale") {
                self.accel_scale[i] = val;
            }
        }
        // Negate the y axis
        self.accel_scale[1] *= -1.0;

        // Get the gyro scale
        for (i, gc) in self.gyro_chan.iter().enumerate() {
            if let Ok(val) = gc.attr_read_float("scale") {
                // Set scale in radians/s
                self.gyro_scale[i] = val;
            }
        }

        // Get the mag scale
        for (i, mc) in self.mag_chan.iter().enumerate() {
            if let Ok(val) = mc.attr_read_float("scale") {
                self.mag_scale[i] = val;
            }
        }
        // Negate the x axis
        self.mag_scale[0] *= -1.0;
        // Negate the y axis
        self.mag_scale[1] *= -1.0;
    }

    fn set_sampling_freq(&self) {
        // Set the acceleration sampling frequency
        for ac in self.accel_chan.iter() {
            ac.attr_write_float("sampling_frequency", 476.0).unwrap();
        }

        // Set the gyro sampling frequency
        for gc in self.gyro_chan.iter() {
            gc.attr_write_float("sampling_frequency", 476.0).unwrap();
        }

        // Set the mag sampling frequency
        for mc in self.mag_chan.iter() {
            mc.attr_write_int("sampling_frequency", 80).unwrap();
        }
    }

    fn get_accel_data(&self) -> Vector3<f64> {
        let mut accel_data = Vector3::new(0.0, 0.0, 0.0);

        if let Ok(val) = self.accel_chan[0].attr_read_int("raw") {
            accel_data.x = (val as f64 - self.accel_calib[0]) * self.accel_scale[0];
        }
        if let Ok(val) = self.accel_chan[1].attr_read_int("raw") {
            accel_data.y = (val as f64 - self.accel_calib[1]) * self.accel_scale[1];
        }
        if let Ok(val) = self.accel_chan[2].attr_read_int("raw") {
            accel_data.z = (val as f64 - self.accel_calib[2]) * self.accel_scale[2];
        }

        accel_data
    }

    fn get_gyro_data(&self) -> Vector3<f64> {
        let mut gyro_data = Vector3::new(0.0, 0.0, 0.0);

        if let Ok(val) = self.gyro_chan[0].attr_read_int("raw") {
            gyro_data.x = val as f64 * self.gyro_scale[0];
        }
        if let Ok(val) = self.gyro_chan[1].attr_read_int("raw") {
            gyro_data.y = val as f64 * self.gyro_scale[1];
        }
        if let Ok(val) = self.gyro_chan[2].attr_read_int("raw") {
            gyro_data.z = val as f64 * self.gyro_scale[2];
        }

        gyro_data
    }

    fn get_9_dofs(&self) -> (Vector3<f64>, Vector3<f64>, Vector3<f64>) {
        let mut mag_filt_input = Vector3::new(0.0, 0.0, 0.0);

        if let Ok(val) = self.mag_chan[0].attr_read_int("raw") {
            mag_filt_input.x = val as f64 * self.mag_scale[0];
        }
        if let Ok(val) = self.mag_chan[1].attr_read_int("raw") {
            mag_filt_input.y = val as f64 * self.mag_scale[1];
        }
        if let Ok(val) = self.mag_chan[2].attr_read_int("raw") {
            mag_filt_input.z = val as f64 * self.mag_scale[2];
        }

        (self.get_accel_data(), self.get_gyro_data(), mag_filt_input)
    }

    fn update_quaternion<'a>(
        &self,
        ahrs: &'a mut ahrs::Madgwick<f64>,
    ) -> &'a nalgebra::Quaternion<f64> {
        let (accel_filt_input, gyro_filt_input, mag_filt_input) = self.get_9_dofs();

        println!(
            "Update Quat: ({:?}, {:?}, {:?})",
            accel_filt_input, gyro_filt_input, mag_filt_input
        );

        // Run inputs through AHRS filter (gyroscope must be radians/s)
        ahrs.update(&gyro_filt_input, &accel_filt_input, &mag_filt_input)
            .unwrap()
    }

    fn generate_inital_quaternion(&self) -> ahrs::Madgwick<f64> {
        // Create AHRS filter
        let mut ahrs = Madgwick::default();

        // TODO: Add prompt
        println!("Make sure sensor axis is lined up with car");
        for _i in 0..10 {
            self.update_quaternion(&mut ahrs);
        }

        // TODO: Convert to prompt
        println!("Move the device to the mount position");
        for _i in 0..50 {
            self.update_quaternion(&mut ahrs);
        }

        ahrs
    }

    fn rotate_accel_data(unit_quat: UnitQuaternion<f64>, accel_data: Vector3<f64>) -> Vector3<f64> {
        unit_quat.transform_vector(&accel_data)
    }

    fn rotate_gyro_data(unit_quat: UnitQuaternion<f64>, gyro_data: Vector3<f64>) -> Vector3<f64> {
        unit_quat.conjugate().transform_vector(&gyro_data)
    }
}

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

    let mut imu_context = ImuContext::new(&ctx);

    imu_context.get_calibration_data();
    imu_context.get_scale_data();

    imu_context.set_sampling_freq();

    // Generate the initial quaternion (to handle unalligned axis)
    let mut ahrs = imu_context.generate_inital_quaternion();

    // Get the mount Quaternion
    let quat_mount = imu_context.update_quaternion(&mut ahrs);

    println!(
        "Euler angles unit_quat_mount: {:?} : {:?}",
        UnitQuaternion::from_quaternion(*quat_mount).euler_angles(),
        *quat_mount,
    );

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
    writeln!(fd, "accel x, accel y, accel z, gyro x, gyro y, gyro z").unwrap();

    while !thread_info.close.lock().unwrap().get() {
        // Get and rotate the acceleration data
        let accel_data = imu_context.get_accel_data();
        let accel_rotated =
            ImuContext::rotate_accel_data(UnitQuaternion::from_quaternion(*quat_mount), accel_data);

        // Write the acceleration data to file
        for data in accel_rotated.iter() {
            write!(fd, "{}", data).unwrap();
            write!(fd, ",").unwrap();
        }

        // Send acceleration data to be drawn on the screen
        thread_info
            .imu_tx
            .send((accel_rotated[0], accel_rotated[1]))
            .unwrap();

        // Get and rotate the gyro data
        // Rotate the data based on the mount quaternion
        let gyro_data = imu_context.get_gyro_data();
        let gyro_rotated =
            ImuContext::rotate_gyro_data(UnitQuaternion::from_quaternion(*quat_mount), gyro_data);

        // Write the gyro data to a file
        for (i, data) in gyro_rotated.iter().enumerate() {
            write!(fd, "{}", data).unwrap();
            if i < 2 {
                write!(fd, ",").unwrap();
            }
        }

        writeln!(fd).unwrap();
    }

    fd.flush().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accel_rotate() {
        let accel_data = Vector3::new(-4.707456, -5.550636, 5.477082);
        let unit_quat = UnitQuaternion::from_euler_angles(-0.7921, 0.5432, 0.07);

        let accel_rotated = ImuContext::rotate_accel_data(unit_quat, accel_data);

        assert_eq!(
            accel_rotated,
            Vector3::new(
                0.0007081578911547481,
                0.0002999181164016207,
                9.108684243057144
            )
        );
    }

    #[test]
    fn test_gyro_rotate() {
        let gyro_data = Vector3::new(-0.571302, 0.377145, 0.171054);
        // let quat_mount = Quaternion::new(
        //     -0.07210097397673415,
        //     0.03503206144556142,
        //     0.040285340134302454,
        //     0.9959675173381631,
        // );
        let unit_quat = UnitQuaternion::from_euler_angles(
            -0.14167536249352172,
            0.07566298807523707,
            0.07548149738661153,
        );

        let gyro_rotated = ImuContext::rotate_gyro_data(unit_quat, gyro_data);

        assert_eq!(
            gyro_rotated,
            Vector3::new(
                -0.5526164393071215,
                0.39664621409393747,
                0.18753839339576528
            )
        );
    }
}
