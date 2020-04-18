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
use ahrs::{Ahrs, Madgwick};
use industrial_io as iio;
use nalgebra::Vector3;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process;

fn create_imu_chans(
    ctx: &iio::Context,
) -> (
    [iio::channel::Channel; 3],
    [iio::channel::Channel; 3],
    [iio::channel::Channel; 3],
) {
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

    (accel_chan, gyro_chan, mag_chan)
}

fn get_calibration_data(
    accel_chan: &[iio::channel::Channel; 3],
    _gyro_chan: &[iio::channel::Channel; 3],
    _mag_chan: &[iio::channel::Channel; 3],
) -> [f64; 3] {
    let mut accel_calib = [0.0, 0.0, 0.0];

    // Get the acceleration calibration offset
    for (i, ac) in accel_chan.iter().enumerate() {
        if let Ok(val) = ac.attr_read_int("calibbias") {
            accel_calib[i] = val as f64;
        }
    }

    accel_calib
}

fn get_scale_data(
    accel_chan: &[iio::channel::Channel; 3],
    gyro_chan: &[iio::channel::Channel; 3],
    mag_chan: &[iio::channel::Channel; 3],
) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let mut accel_scale = [1.0, 1.0, 1.0];
    let mut gyro_scale = [1.0, 1.0, 1.0];
    let mut mag_scale = [1.0, 1.0, 1.0];

    // Get the acceleration scale
    for (i, ac) in accel_chan.iter().enumerate() {
        if let Ok(val) = ac.attr_read_float("scale") {
            accel_scale[i] = val;
        }
    }
    // Negate the y axis
    accel_scale[1] = accel_scale[1] * -1.0;

    // Get the gyro scale
    for (i, ac) in gyro_chan.iter().enumerate() {
        if let Ok(val) = ac.attr_read_float("scale") {
            gyro_scale[i] = val;
        }
    }
    // Negate the y axis
    gyro_scale[1] = gyro_scale[1] * -1.0;

    // Get the mag scale
    for (i, mc) in mag_chan.iter().enumerate() {
        if let Ok(val) = mc.attr_read_float("scale") {
            mag_scale[i] = val;
        }
    }
    // Negate the y axis
    mag_scale[1] = mag_scale[1] * -1.0;

    (accel_scale, gyro_scale, mag_scale)
}
fn set_sampling_freq(
    accel_chan: &[iio::channel::Channel; 3],
    gyro_chan: &[iio::channel::Channel; 3],
    mag_chan: &[iio::channel::Channel; 3],
) {
    // Set the acceleration sampling frequency
    for ac in accel_chan.iter() {
        ac.attr_write_float("sampling_frequency", 952.0).unwrap();
    }

    // Set the gyro sampling frequency
    for gc in gyro_chan.iter() {
        gc.attr_write_float("sampling_frequency", 952.0).unwrap();
    }

    // Set the mag sampling frequency
    for mc in mag_chan.iter() {
        mc.attr_write_int("sampling_frequency", 80).unwrap();
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

    let (accel_chan, gyro_chan, mag_chan) = create_imu_chans(&ctx);

    let accel_calib = get_calibration_data(&accel_chan, &gyro_chan, &mag_chan);
    let (accel_scale, gyro_scale, mag_scale) = get_scale_data(&accel_chan, &gyro_chan, &mag_chan);

    set_sampling_freq(&accel_chan, &gyro_chan, &mag_chan);

    // TODO: Add prompt
    // Make sure sensor axis is lined up with car

    // Create AHRS filter
    let mut ahrs = Madgwick::default();

    let mut accel_filt_input = Vector3::new(0.0, 0.0, 0.0);
    if let Ok(val) = accel_chan[0].attr_read_int("raw") {
        accel_filt_input.x = (val as f64 - accel_calib[0]) * accel_scale[0];
    }
    if let Ok(val) = accel_chan[1].attr_read_int("raw") {
        accel_filt_input.y = (val as f64 - accel_calib[1]) * accel_scale[1];
    }
    if let Ok(val) = accel_chan[2].attr_read_int("raw") {
        accel_filt_input.z = (val as f64 - accel_calib[2]) * accel_scale[2];
    }

    let mut gyro_filt_input = Vector3::new(0.0, 0.0, 0.0);
    if let Ok(val) = gyro_chan[0].attr_read_int("raw") {
        gyro_filt_input.x = val as f64 * accel_scale[0];
    }
    if let Ok(val) = gyro_chan[1].attr_read_int("raw") {
        gyro_filt_input.y = val as f64 * accel_scale[1];
    }
    if let Ok(val) = gyro_chan[2].attr_read_int("raw") {
        gyro_filt_input.z = val as f64 * accel_scale[2];
    }

    let mut mag_filt_input = Vector3::new(0.0, 0.0, 0.0);
    if let Ok(val) = mag_chan[0].attr_read_int("raw") {
        mag_filt_input.x = val as f64 * mag_scale[0];
    }
    if let Ok(val) = mag_chan[1].attr_read_int("raw") {
        mag_filt_input.y = val as f64 * mag_scale[1];
    }
    if let Ok(val) = mag_chan[2].attr_read_int("raw") {
        mag_filt_input.z = val as f64 * mag_scale[2];
    }

    // Run inputs through AHRS filter (gyroscope must be radians/s)
    let quat_car = ahrs
        .update(&(gyro_filt_input), &accel_filt_input, &mag_filt_input)
        .unwrap()
        .clone();

    println!("The first (lined up) quaternion is: {}", quat_car);
    println!(
        "Euler angles quat_car: {:?}",
        nalgebra::geometry::UnitQuaternion::from_quaternion(quat_car).euler_angles()
    );
    println!(
        "Rotation Matrix angles quat_car: {:?}",
        nalgebra::geometry::UnitQuaternion::from_quaternion(quat_car).to_rotation_matrix()
    );

    println!("Move the device to the mount position");

    for _i in 0..60 {
        if let Ok(val) = accel_chan[0].attr_read_int("raw") {
            accel_filt_input.x = (val as f64 - accel_calib[0]) * accel_scale[0];
        }
        if let Ok(val) = accel_chan[1].attr_read_int("raw") {
            accel_filt_input.y = (val as f64 - accel_calib[1]) * accel_scale[1];
        }
        if let Ok(val) = accel_chan[2].attr_read_int("raw") {
            accel_filt_input.z = (val as f64 - accel_calib[2]) * accel_scale[2];
        }

        if let Ok(val) = gyro_chan[0].attr_read_int("raw") {
            gyro_filt_input.x = val as f64 * accel_scale[0];
        }
        if let Ok(val) = gyro_chan[1].attr_read_int("raw") {
            gyro_filt_input.y = val as f64 * accel_scale[1];
        }
        if let Ok(val) = gyro_chan[2].attr_read_int("raw") {
            gyro_filt_input.z = val as f64 * accel_scale[2];
        }

        if let Ok(val) = mag_chan[0].attr_read_int("raw") {
            mag_filt_input.x = val as f64 * mag_scale[0];
        }
        if let Ok(val) = mag_chan[1].attr_read_int("raw") {
            mag_filt_input.y = val as f64 * mag_scale[1];
        }
        if let Ok(val) = mag_chan[2].attr_read_int("raw") {
            mag_filt_input.z = val as f64 * mag_scale[2];
        }

        // Run inputs through AHRS filter (gyroscope must be radians/s)
        ahrs.update(&(gyro_filt_input), &accel_filt_input, &mag_filt_input)
            .unwrap();
    }

    println!("Calculating the mount quaternion");

    // Run inputs through AHRS filter (gyroscope must be radians/s)
    let quat_mount = ahrs
        .update(&(gyro_filt_input), &accel_filt_input, &mag_filt_input)
        .unwrap()
        .clone();

    println!("The mounted quaternion is: {}", quat_mount);

    let quat_diff_1 = quat_car - quat_mount;
    let quat_diff_2 = quat_mount - quat_car;

    println!("The diff quaternion 1 is: {}", quat_diff_1);
    println!("The diff quaternion 2 is: {}", quat_diff_2);

    let unit_quat_mount_1 = nalgebra::geometry::UnitQuaternion::from_quaternion(quat_mount);
    println!(
        "Euler angles unit_quat_mount_1: {:?}",
        unit_quat_mount_1.euler_angles()
    );

    println!(
        "Rotation Matrix unit_quat_mount_1: {:?}",
        unit_quat_mount_1.to_rotation_matrix()
    );
    let unit_quat_mount_diff_1 = nalgebra::geometry::UnitQuaternion::from_quaternion(quat_diff_1);
    println!(
        "Euler angles unit_quat_mount_diff_1: {:?}",
        unit_quat_mount_diff_1.euler_angles()
    );
    let unit_quat_mount_diff_2 = nalgebra::geometry::UnitQuaternion::from_quaternion(quat_diff_2);
    println!(
        "Euler angles unit_quat_mount_diff_2: {:?}",
        unit_quat_mount_diff_2.euler_angles()
    );
    println!(
        "Rotation Matrix unit_quat_mount_diff_2: {:?}",
        unit_quat_mount_diff_2.to_rotation_matrix()
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
    write!(fd, "accel x, accel y, accel z, gyro x, gyro y, gyro z\n").unwrap();

    while !thread_info.close.lock().unwrap().get() {
        let mut accel = Vector3::new(0.0, 0.0, 0.0);

        for (i, ac) in accel_chan.iter().enumerate() {
            if let Ok(val) = ac.attr_read_int("raw") {
                accel[i] = (val as f64 - accel_calib[i]) * accel_scale[i];

                write!(fd, "{}", accel[i]).unwrap();
            }
            write!(fd, ",").unwrap();
        }

        println!("accel: x: {}; y: {}", accel[0], accel[1]);

        let accel_quat = nalgebra::geometry::Quaternion::from_imag(accel);

        println!("accel_quat: {:?}", accel_quat);

        let accel_rotated = unit_quat_mount_1.transform_vector(&accel);
        let accel_rotated_unit = unit_quat_mount_1.quaternion()
            * accel_quat
            * unit_quat_mount_1.quaternion().conjugate();

        let accel_rotated_2 =
            nalgebra::geometry::UnitQuaternion::from_quaternion(quat_mount.conjugate())
                .transform_vector(&accel);
        let unit_accel_rotated_2 = unit_quat_mount_1.conjugate().transform_vector(&accel);

        println!(
            "accel_rotated_1: x: {}; y: {}; z: {}",
            accel_rotated[0], accel_rotated[1], accel_rotated[2]
        );
        println!(
            "accel_rotated_unit: x: {}; y: {}; z: {}",
            accel_rotated_unit[0], accel_rotated_unit[1], accel_rotated_unit[2]
        );
        println!(
            "accel_rotated_2: x: {}; y: {}; z: {}",
            accel_rotated_2[0], accel_rotated_2[1], accel_rotated_2[2]
        );
        println!(
            "unit_accel_rotated_2: x: {}; y: {}; z: {}",
            unit_accel_rotated_2[0], unit_accel_rotated_2[1], unit_accel_rotated_2[2]
        );

        let accel_rotated_diff_1 = quat_diff_1 * accel_quat * quat_diff_1.conjugate();
        let accel_rotated_diff_1_2 = unit_quat_mount_diff_1.quaternion()
            * accel_quat
            * unit_quat_mount_diff_1.quaternion().conjugate();

        println!(
            "accel_rotated_diff_1: x: {}; y: {}; z: {}",
            accel_rotated_diff_1[0], accel_rotated_diff_1[1], accel_rotated_diff_1[2]
        );
        println!(
            "accel_rotated_diff_1_2: x: {}; y: {}; z: {}",
            accel_rotated_diff_1_2[0], accel_rotated_diff_1_2[1], accel_rotated_diff_1_2[2]
        );

        let accel_rotated_diff_2 = quat_diff_1 * accel_quat * quat_diff_1.conjugate();
        let accel_rotated_diff_2_2 = unit_quat_mount_diff_1.quaternion()
            * accel_quat
            * unit_quat_mount_diff_1.quaternion().conjugate();

        println!(
            "accel_rotated_diff_2: x: {}; y: {}; z: {}",
            accel_rotated_diff_2[0], accel_rotated_diff_2[1], accel_rotated_diff_2[2]
        );
        println!(
            "accel_rotated_diff_2_2: x: {}; y: {}; z: {}",
            accel_rotated_diff_2_2[0], accel_rotated_diff_2_2[1], accel_rotated_diff_2_2[2]
        );
        println!("");

        thread_info
            .imu_tx
            .send((accel_rotated[0], accel_rotated[1]))
            .unwrap();

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
    }

    fd.flush().unwrap();
}
