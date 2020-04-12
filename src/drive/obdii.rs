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

extern crate cpython;
use crate::drive::drive::*;
use chrono::prelude::*;
use cpython::{PyResult, Python};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
pub enum OBDIICommandType {
    Rpm,
    Throttle,
    EngineLoad,
    TimingAdv,
    Maf,
    CoolantTemp,
    IntakeTemp,
    ShortFuelT1,
    LongFuelT1,
    FuelStatus,
}

#[derive(PartialEq)]
enum PythonReturns {
    Float,
    Long,
    PyStr,
}

struct OBDIICommands {
    command: OBDIICommandType,
    com_string: String,
    ret: PythonReturns,
}

#[derive(Clone, Copy)]
pub enum OBDIIFuelStatus {
    OpenLoopTemp,
    ClosedLoopO2Sense,
    OpenLoopLoad,
    OpenLoopFailure,
    ClosedLoopFault,
}

pub union PythonValues {
    pub float: f64,
    pub long: u32,
    pub fuel_status: OBDIIFuelStatus,
}

pub struct OBDIIData {
    pub command: OBDIICommandType,
    pub val: PythonValues,
}

pub fn obdii_thread(thread_info: ThreadingRef, file_name: &mut PathBuf) -> PyResult<()> {
    let gli = Python::acquire_gil();
    let py = gli.python();

    let commands: [OBDIICommands; 10] = [
        OBDIICommands {
            command: OBDIICommandType::Rpm,
            com_string: "RPM".to_string(),
            ret: PythonReturns::Float,
        },
        OBDIICommands {
            command: OBDIICommandType::Throttle,
            com_string: "THROTTLE_POS".to_string(),
            ret: PythonReturns::Float,
        },
        OBDIICommands {
            command: OBDIICommandType::EngineLoad,
            com_string: "ENGINE_LOAD".to_string(),
            ret: PythonReturns::Float,
        },
        OBDIICommands {
            command: OBDIICommandType::TimingAdv,
            com_string: "TIMING_ADVANCE".to_string(),
            ret: PythonReturns::Float,
        },
        OBDIICommands {
            command: OBDIICommandType::Maf,
            com_string: "MAF".to_string(),
            ret: PythonReturns::Float,
        },
        OBDIICommands {
            command: OBDIICommandType::CoolantTemp,
            com_string: "COOLANT_TEMP".to_string(),
            ret: PythonReturns::Long,
        },
        OBDIICommands {
            command: OBDIICommandType::IntakeTemp,
            com_string: "INTAKE_TEMP".to_string(),
            ret: PythonReturns::Long,
        },
        OBDIICommands {
            command: OBDIICommandType::ShortFuelT1,
            com_string: "SHORT_FUEL_TRIM_1".to_string(),
            ret: PythonReturns::Float,
        },
        OBDIICommands {
            command: OBDIICommandType::LongFuelT1,
            com_string: "LONG_FUEL_TRIM_1".to_string(),
            ret: PythonReturns::Float,
        },
        OBDIICommands {
            command: OBDIICommandType::FuelStatus,
            com_string: "FUEL_STATUS".to_string(),
            ret: PythonReturns::PyStr,
        },
    ];

    let mut name = file_name.file_stem().unwrap().to_str().unwrap().to_string();

    name.push_str("-obdii.cvs");

    file_name.pop();
    file_name.push(name);

    let mut obdii_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&file_name);

    let fd = obdii_file.as_mut().unwrap();

    write!(fd, "Time").unwrap();

    for command in commands.iter() {
        write!(fd, ",{}", command.com_string).unwrap();
    }

    write!(fd, "\n").unwrap();

    while !thread_info.close.lock().unwrap().get() {
        let pyobd_res;

        loop {
            let res = py.import("obdii_connect");

            match res {
                Ok(r) => {
                    pyobd_res = r;
                    break;
                }
                Err(e) => {
                    println!("Unable to conect to OBDII {:?}", e);
                    thread::sleep(Duration::from_secs(10));
                    if thread_info.close.lock().unwrap().get() {
                        return Ok(());
                    } else {
                        continue;
                    }
                }
            }
        }

        while !thread_info.close.lock().unwrap().get() {
            write!(fd, "{}", Utc::now()).unwrap();

            for command in commands.iter() {
                let py_ret = pyobd_res.call(py, "c_get_data", (&command.com_string,), None)?;

                let data: OBDIIData;

                if command.ret == PythonReturns::Float {
                    let ret_res = py_ret.extract(py);

                    let ret: f64;
                    match ret_res {
                        Ok(r) => {
                            ret = r;
                        }
                        Err(e) => {
                            println!(
                                "{}: Error in extracting the float: {:?}; {:?}",
                                command.com_string, py_ret, e
                            );
                            continue;
                        }
                    }

                    write!(fd, ",{}", ret).unwrap();

                    data = OBDIIData {
                        command: command.command,
                        val: PythonValues { float: ret },
                    };
                } else if command.ret == PythonReturns::Long {
                    let ret_res = py_ret.extract(py);

                    let ret: u32;
                    match ret_res {
                        Ok(r) => {
                            ret = r;
                        }
                        Err(e) => {
                            println!(
                                "{}: Error in extracting the long: {:?}; {:?}",
                                command.com_string, py_ret, e
                            );
                            continue;
                        }
                    }

                    write!(fd, ",{}", ret).unwrap();

                    data = OBDIIData {
                        command: command.command,
                        val: PythonValues { long: ret },
                    };
                } else {
                    let ret_res = py_ret.extract(py);

                    let ret: String;
                    match ret_res {
                        Ok(r) => {
                            ret = r;
                        }
                        Err(e) => {
                            println!(
                                "{}: Error in extracting the string: {:?}; {:?}",
                                command.com_string, py_ret, e
                            );
                            continue;
                        }
                    }

                    write!(fd, ",{}", ret).unwrap();

                    let mut fuel_status = OBDIIFuelStatus::OpenLoopTemp;

                    if ret.eq("Open loop due to insufficient engine temperature") {
                        fuel_status = OBDIIFuelStatus::OpenLoopTemp;
                    } else if ret.eq("Closed loop, using oxygen sensor feedback to determine fuel mix") {
                        fuel_status = OBDIIFuelStatus::ClosedLoopO2Sense;
                    } else if ret.eq("Open loop due to engine load OR fuel cut due to deceleration") {
                        fuel_status = OBDIIFuelStatus::OpenLoopLoad;
                    } else if ret.eq("Open loop due to system failure") {
                        fuel_status = OBDIIFuelStatus::OpenLoopFailure;
                    } else if ret.eq("Closed loop, using at least one oxygen sensor but there is a fault in the feedback system") {
                        fuel_status = OBDIIFuelStatus::ClosedLoopFault;
                    }

                    data = OBDIIData {
                        command: command.command,
                        val: PythonValues {
                            fuel_status: fuel_status,
                        },
                    };
                }

                thread_info.obdii_tx.send(data).unwrap();
            }

            write!(fd, "\n").unwrap();
        }
    }

    fd.flush().unwrap();

    Ok(())
}
