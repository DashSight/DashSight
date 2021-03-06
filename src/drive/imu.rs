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

pub const IMU_SAMPLE_FREQ: f64 = 60.0;

struct ImuContext {
    accel_chan: Option<[iio::channel::Channel; 3]>,
    gyro_chan: Option<[iio::channel::Channel; 3]>,
    mag_chan: Option<[iio::channel::Channel; 3]>,

    accel_calib: [f64; 3],

    accel_scale: [f64; 3],
    gyro_scale: [f64; 3],
    mag_scale: [f64; 3],

    max_g_force: f64,

    rotation_unit_quat: Option<UnitQuaternion<f64>>,
}

impl Default for ImuContext {
    fn default() -> Self {
        ImuContext {
            accel_chan: None,
            gyro_chan: None,
            mag_chan: None,
            accel_calib: [0.0, 0.0, 0.0],
            accel_scale: [0.0, 0.0, 0.0],
            gyro_scale: [0.0, 0.0, 0.0],
            mag_scale: [0.0, 0.0, 0.0],
            max_g_force: 0.0,
            rotation_unit_quat: None,
        }
    }
}

impl ImuContext {
    fn new(ctx: &iio::Context) -> Result<ImuContext, String> {
        // Create the IMU accel device
        let imu_name = "lsm9ds1-imu_accel";
        let imu_accel_dev;
        match ctx.find_device(imu_name) {
            Some(dev) => {
                imu_accel_dev = dev;
            }
            None => {
                return Err(format!("Error opening device: {}", imu_name));
            }
        }

        // Create the IMU gyro device
        let imu_name = "lsm9ds1-imu_gyro";
        let imu_gyro_dev;
        match ctx.find_device(imu_name) {
            Some(dev) => {
                imu_gyro_dev = dev;
            }
            None => {
                return Err(format!("Error opening device: {}", imu_name));
            }
        }

        // Create the IMU mag device
        let imu_name = "lsm9ds1_magn";
        let imu_mag_dev;
        match ctx.find_device(imu_name) {
            Some(dev) => {
                imu_mag_dev = dev;
            }
            None => {
                return Err(format!("Error opening device: {}", imu_name));
            }
        }

        // Get the IMU acceleration channels
        let x_accel_chan = imu_accel_dev
            .find_channel("accel_x", false)
            .unwrap_or_else(|| {
                panic!("No 'accel_x' channel on this device");
            });
        let y_accel_chan = imu_accel_dev
            .find_channel("accel_y", false)
            .unwrap_or_else(|| {
                panic!("No 'accel_y' channel on this device");
            });
        let z_accel_chan = imu_accel_dev
            .find_channel("accel_z", false)
            .unwrap_or_else(|| {
                panic!("No 'accel_z' channel on this device");
            });

        // Get the IMU gyro channels
        let x_gyro_chan = imu_gyro_dev
            .find_channel("anglvel_x", false)
            .unwrap_or_else(|| {
                panic!("No 'anglvel_x' channel on this device");
            });
        let y_gyro_chan = imu_gyro_dev
            .find_channel("anglvel_y", false)
            .unwrap_or_else(|| {
                panic!("No 'anglvel_y' channel on this device");
            });
        let z_gyro_chan = imu_gyro_dev
            .find_channel("anglvel_z", false)
            .unwrap_or_else(|| {
                panic!("No 'anglvel_z' channel on this device");
            });

        // Get the IMU mag channels
        let x_mag_chan = imu_mag_dev
            .find_channel("magn_x", false)
            .unwrap_or_else(|| {
                panic!("No 'magn_x' channel on this device");
            });
        let y_mag_chan = imu_mag_dev
            .find_channel("magn_y", false)
            .unwrap_or_else(|| {
                panic!("No 'magn_y' channel on this device");
            });
        let z_mag_chan = imu_mag_dev
            .find_channel("magn_z", false)
            .unwrap_or_else(|| {
                panic!("No 'magn_z' channel on this device");
            });

        let accel_chan: [iio::channel::Channel; 3] = [x_accel_chan, y_accel_chan, z_accel_chan];
        let gyro_chan: [iio::channel::Channel; 3] = [x_gyro_chan, y_gyro_chan, z_gyro_chan];
        let mag_chan: [iio::channel::Channel; 3] = [x_mag_chan, y_mag_chan, z_mag_chan];

        Ok(ImuContext {
            accel_chan: Some(accel_chan),
            gyro_chan: Some(gyro_chan),
            mag_chan: Some(mag_chan),
            accel_calib: [0.0, 0.0, 0.0],
            accel_scale: [0.0, 0.0, 0.0],
            gyro_scale: [0.0, 0.0, 0.0],
            mag_scale: [0.0, 0.0, 0.0],
            max_g_force: 0.0,
            rotation_unit_quat: None,
        })
    }

    fn get_calibration_data(&mut self) {
        // Get the acceleration calibration offset
        for (i, ac) in self.accel_chan.as_ref().unwrap().iter().enumerate() {
            if let Ok(val) = ac.attr_read_int("calibbias") {
                self.accel_calib[i] = val as f64;
            }
        }
    }

    fn get_scale_data(&mut self) {
        // Get the acceleration scale
        for (i, ac) in self.accel_chan.as_ref().unwrap().iter().enumerate() {
            if let Ok(val) = ac.attr_read_float("scale") {
                self.accel_scale[i] = val;
            }
        }
        // Negate the y axis
        self.accel_scale[1] *= -1.0;

        // Get the gyro scale
        for (i, gc) in self.gyro_chan.as_ref().unwrap().iter().enumerate() {
            if let Ok(val) = gc.attr_read_float("scale") {
                // Set scale in radians/s
                self.gyro_scale[i] = val;
            }
        }

        // Get the mag scale
        for (i, mc) in self.mag_chan.as_ref().unwrap().iter().enumerate() {
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
        for ac in self.accel_chan.as_ref().unwrap().iter() {
            ac.attr_write_float("sampling_frequency", 476.0).unwrap();
        }

        // Set the gyro sampling frequency
        for gc in self.gyro_chan.as_ref().unwrap().iter() {
            gc.attr_write_float("sampling_frequency", 476.0).unwrap();
        }

        // Set the mag sampling frequency
        for mc in self.mag_chan.as_ref().unwrap().iter() {
            mc.attr_write_int("sampling_frequency", 80).unwrap();
        }
    }

    fn get_accel_data(&self) -> Vector3<f64> {
        let mut accel_data = Vector3::new(0.0, 0.0, 0.0);

        if let Ok(val) = self.accel_chan.as_ref().unwrap()[0].attr_read_int("raw") {
            accel_data.x = (val as f64 - self.accel_calib[0]) * self.accel_scale[0];
        }
        if let Ok(val) = self.accel_chan.as_ref().unwrap()[1].attr_read_int("raw") {
            accel_data.y = (val as f64 - self.accel_calib[1]) * self.accel_scale[1];
        }
        if let Ok(val) = self.accel_chan.as_ref().unwrap()[2].attr_read_int("raw") {
            accel_data.z = (val as f64 - self.accel_calib[2]) * self.accel_scale[2];
        }

        accel_data
    }

    fn get_gyro_data(&self) -> Vector3<f64> {
        let mut gyro_data = Vector3::new(0.0, 0.0, 0.0);

        if let Ok(val) = self.gyro_chan.as_ref().unwrap()[0].attr_read_int("raw") {
            gyro_data.x = val as f64 * self.gyro_scale[0];
        }
        if let Ok(val) = self.gyro_chan.as_ref().unwrap()[1].attr_read_int("raw") {
            gyro_data.y = val as f64 * self.gyro_scale[1];
        }
        if let Ok(val) = self.gyro_chan.as_ref().unwrap()[2].attr_read_int("raw") {
            gyro_data.z = val as f64 * self.gyro_scale[2];
        }

        gyro_data
    }

    fn get_mag_data(&self) -> Vector3<f64> {
        let mut mag_data = Vector3::new(0.0, 0.0, 0.0);

        if let Ok(val) = self.mag_chan.as_ref().unwrap()[0].attr_read_int("raw") {
            mag_data.x = val as f64 * self.mag_scale[0];
        }
        if let Ok(val) = self.mag_chan.as_ref().unwrap()[1].attr_read_int("raw") {
            mag_data.y = val as f64 * self.mag_scale[1];
        }
        if let Ok(val) = self.mag_chan.as_ref().unwrap()[2].attr_read_int("raw") {
            mag_data.z = val as f64 * self.mag_scale[2];
        }

        mag_data
    }

    fn calibrate_rotation_matrix(&mut self, accel_data: &Vector3<f64>) {
        let gravity = Vector3::new(0.0, 0.0, 9.8);
        self.rotation_unit_quat =
            Some(UnitQuaternion::rotation_between(&accel_data, &gravity).unwrap());
    }

    /// Rotate the acceleration data by the already calibrated rotation matrix
    fn rotate_data(&self, accel_data: &Vector3<f64>) -> Vector3<f64> {
        match self.rotation_unit_quat {
            Some(rotate) => rotate.transform_vector(accel_data),
            None => *accel_data,
        }
    }
}

pub fn imu_thread(
    thread_info: ThreadingRef,
    imu_tx: std::sync::mpsc::Sender<(f64, f64, Option<f64>, Option<f64>)>,
    imu_page_tx: std::sync::mpsc::Sender<(f64, f64, Option<f64>, Option<f64>)>,
    file_name: &mut PathBuf,
) {
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

    let mut imu_context;
    match ImuContext::new(&ctx) {
        Ok(imu) => {
            imu_context = imu;
        }
        Err(e) => {
            println!("{:?}", e);
            return;
        }
    }

    imu_context.get_calibration_data();
    imu_context.get_scale_data();

    imu_context.set_sampling_freq();

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

    let mut ahrs = Madgwick::new(IMU_SAMPLE_FREQ / 1000.0, 0.1);

    // Write the CVS headers
    writeln!(fd, "accel x, accel y, accel z, gyro x, gyro y, gyro z").unwrap();

    while !thread_info.close.lock().unwrap().get() {
        if thread_info.calibrate.lock().unwrap().get() {
            println!("Calibrating, make sure there is no acceleration");
            let accel_data = imu_context.get_accel_data();
            imu_context.calibrate_rotation_matrix(&accel_data);
            ahrs = Madgwick::new_with_quat(
                IMU_SAMPLE_FREQ / 1000.0,
                0.1,
                *imu_context.rotation_unit_quat.unwrap().quaternion(),
            );
            thread_info.calibrate.lock().unwrap().set(false);
        }

        // Get and rotate the acceleration data
        let accel_data = imu_context.get_accel_data();
        let accel_rotated = imu_context.rotate_data(&accel_data);

        // Write the acceleration data to file
        for data in accel_rotated.iter() {
            write!(fd, "{}", data).unwrap();
            write!(fd, ",").unwrap();
        }

        // Calculate absolute G force in X and Y
        let g_force = accel_rotated[0].powi(2) + accel_rotated[1].powi(2);
        let g_force = g_force.sqrt() / 9.8;

        if g_force > imu_context.max_g_force {
            imu_context.max_g_force = g_force;
        }

        // Send acceleration data to be drawn on the screen
        imu_tx
            .send((accel_rotated[0], accel_rotated[1], None, None))
            .unwrap();
        imu_page_tx
            .send((
                accel_rotated[0],
                accel_rotated[1],
                Some(g_force),
                Some(imu_context.max_g_force),
            ))
            .unwrap();

        // Get and rotate the gyro data
        // Rotate the data based on the mount quaternion
        let gyro_data = imu_context.get_gyro_data();
        let gyro_rotated = imu_context.rotate_data(&gyro_data);

        // Write the gyro data to a file
        for (i, data) in gyro_rotated.iter().enumerate() {
            write!(fd, "{}", data).unwrap();
            if i < 2 {
                write!(fd, ",").unwrap();
            }
        }

        writeln!(fd).unwrap();

        let mag_data = imu_context.get_mag_data();

        let quat = ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();
        let _unit_quat = UnitQuaternion::from_quaternion(*quat);

        let quat_rotated = ahrs
            .update(&gyro_rotated, &accel_rotated, &mag_data)
            .unwrap();
        let _unit_quat_rotated = UnitQuaternion::from_quaternion(*quat_rotated);

        // println!(
        //     "({:?}, {:?}, {:?}) -> {:?} and {:?}",
        //     gyro_data,
        //     accel_data,
        //     mag_data,
        //     unit_quat.euler_angles(),
        //     unit_quat_rotated.euler_angles()
        // );
    }

    fd.flush().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Tests generating a rotation matrix and rotating acceleration data
    fn test_accel_rotate() {
        let mut imu_context = ImuContext::default();

        let accel_data = Vector3::new(-4.707456, -5.550636, 5.477082);

        imu_context.calibrate_rotation_matrix(&accel_data);

        assert_eq!(
            imu_context.rotation_unit_quat.unwrap().euler_angles(),
            (
                -0.7920679847762988,
                0.5431201418356281,
                -0.23180120006535404
            )
        );

        let accel_rotated = imu_context.rotate_data(&accel_data);

        assert_eq!(
            accel_rotated,
            Vector3::new(0.0, -0.0000000000000008881784197001252, 9.108684275522783)
        );
    }

    #[test]
    /// Tests rotating acceleration data without any rotation matrix
    fn test_accel_empty_rotate() {
        let imu_context = ImuContext::default();

        let accel_data = Vector3::new(-4.707456, -5.550636, 5.477082);

        let accel_rotated = imu_context.rotate_data(&accel_data);

        assert_eq!(accel_rotated, Vector3::new(-4.707456, -5.550636, 5.477082));
    }

    #[test]
    /// Tests generating a Quaternion for Movement
    fn test_quat_gen() {
        let mut ahrs = Madgwick::new(IMU_SAMPLE_FREQ / 1000.0, 0.1);

        let gyro_data = Vector3::new(-0.405909, 0.5429970000000001, 0.150093);
        let accel_data = Vector3::new(-0.057408, -0.001794, 8.52748);
        let mag_data = Vector3::new(-0.036062, -0.688974, -0.205568);

        let quat = ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();
        let unit_quat = UnitQuaternion::from_quaternion(quat.clone());

        assert_eq!(
            unit_quat.euler_angles(),
            (
                -0.020778349488399357,
                0.02939925866524318,
                0.019733896959950713
            )
        );

        let gyro_data = Vector3::new(-0.622251, 0.45027900000000004, 0.16784100000000002);
        let accel_data = Vector3::new(-0.050232, -0.066378, 8.537646);
        let mag_data = Vector3::new(-0.038398, -0.70007, -0.19637);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.648108, 0.340731, 0.187731);
        let accel_data = Vector3::new(-0.004784, -0.0041860000000000005, 8.525088);
        let mag_data = Vector3::new(-0.034602, -0.700946, -0.199436);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.631125, 0.486846, 0.14688);
        let accel_data = Vector3::new(-0.01794, 0.005382, 8.514324);
        let mag_data = Vector3::new(-0.03431, -0.698026, -0.208196);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.6175080000000001, 0.377604, 0.131274);
        let accel_data = Vector3::new(-0.02093, -0.013156000000000001, 8.550204);
        let mag_data = Vector3::new(-0.032558, -0.6939379999999999, -0.204254);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.587214, 0.570843, 0.13617);
        let accel_data = Vector3::new(-0.029302, -0.029302, 8.544224);
        let mag_data = Vector3::new(-0.033434, -0.696274, -0.198414);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.507654, 0.228276, 0.16156800000000002);
        let accel_data = Vector3::new(-0.035282, -0.037076, 8.550802000000001);
        let mag_data = Vector3::new(-0.036938, -0.696274, -0.202356);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.66861, 0.46512000000000003, 0.16845300000000002);
        let accel_data = Vector3::new(-0.030498, -0.01196, 8.551998);
        let mag_data = Vector3::new(-0.029054, -0.69277, -0.19125999999999999);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.5454450000000001, 0.337977, 0.201501);
        let accel_data = Vector3::new(-0.016146, -0.000598, 8.571732);
        let mag_data = Vector3::new(-0.027594, -0.69277, -0.201918);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.681768, 0.386478, 0.13464);
        let accel_data = Vector3::new(-0.021528, -0.008372000000000001, 8.554988);
        let mag_data = Vector3::new(-0.034894, -0.69277, -0.203816);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.476748, 0.52479, 0.153153);
        let accel_data = Vector3::new(-0.025714, -0.034086, 8.571732);
        let mag_data = Vector3::new(-0.028762, -0.694814, -0.20586);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.649332, 0.5347350000000001, 0.20349);
        let accel_data = Vector3::new(-0.031096, -0.016146, 8.565154);
        let mag_data = Vector3::new(-0.037814, -0.6994859999999999, -0.198706);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.6306660000000001, 0.364599, 0.200583);
        let accel_data = Vector3::new(-0.025116, -0.02093, 8.55439);
        let mag_data = Vector3::new(-0.033434, -0.6994859999999999, -0.20469199999999999);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.65484, 0.368271, 0.200277);
        let accel_data = Vector3::new(-0.034684, -0.013754, 8.574124);
        let mag_data = Vector3::new(-0.035186, -0.693354, -0.19856);

        let quat = ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();
        let unit_quat = UnitQuaternion::from_quaternion(quat.clone());

        assert_eq!(
            unit_quat.euler_angles(),
            (
                -0.44645549363939224,
                0.3454127251838817,
                0.22681998416182417
            )
        );

        let gyro_data = Vector3::new(-0.550647, 0.55692, 0.16065000000000002);
        let accel_data = Vector3::new(-0.034684, -0.026312000000000002, 8.56037);
        let mag_data = Vector3::new(-0.031974, -0.703866, -0.195786);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.26759700000000003, 0.463743, 0.156978);
        let accel_data = Vector3::new(-0.024518, 0.033488000000000004, 8.574722);
        let mag_data = Vector3::new(-0.032266, -0.695106, -0.20221);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.62424, 0.561816, 0.171513);
        let accel_data = Vector3::new(-0.071162, -0.089102, 8.630934);
        let mag_data = Vector3::new(-0.03431, -0.694814, -0.205422);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.6820740000000001, 0.342414, 0.199512);
        let accel_data = Vector3::new(0.110032, -0.2392, 8.696714);
        let mag_data = Vector3::new(-0.03723, -0.69131, -0.213598);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.6289830000000001, 0.321453, 0.140148);
        let accel_data = Vector3::new(-0.502918, -0.359996, 9.676238);
        let mag_data = Vector3::new(-0.045698, -0.655978, -0.258566);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.6367860000000001, 0.33858900000000003, 0.145044);
        let accel_data = Vector3::new(-1.095536, -1.561378, 9.094982);
        let mag_data = Vector3::new(-0.046574, -0.623566, -0.285284);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.6158250000000001, 0.523872, 0.149481);
        let accel_data = Vector3::new(-2.2807720000000002, -2.05712, 7.964762);
        let mag_data = Vector3::new(-0.019418, -0.610134, -0.29419);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.606951, 0.5071950000000001, 0.200583);
        let accel_data = Vector3::new(-3.023488, -2.243696, 7.7877540000000005);
        let mag_data = Vector3::new(0.002774, -0.596702, -0.300614);

        ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();

        let gyro_data = Vector3::new(-0.6009840000000001, 0.32436000000000004, 0.165699);
        let accel_data = Vector3::new(-3.34581, -2.450006, 8.162700000000001);
        let mag_data = Vector3::new(0.012118, -0.582102, -0.31317);

        let quat = ahrs.update(&gyro_data, &accel_data, &mag_data).unwrap();
        let unit_quat = UnitQuaternion::from_quaternion(quat.clone());

        assert_eq!(
            unit_quat.euler_angles(),
            (-0.7509667959065289, 0.5823530551171815, 0.28248598214200316)
        );
    }
}
