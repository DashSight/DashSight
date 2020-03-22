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
use cpython::{PyDict, PyResult, Python};
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

pub fn obdii_thread(thread_info: ThreadingRef) -> PyResult<()> {
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
            ret: PythonReturns::Long,
        },
        OBDIICommands {
            command: OBDIICommandType::LongFuelT1,
            com_string: "LONG_FUEL_TRIM_1".to_string(),
            ret: PythonReturns::Long,
        },
        OBDIICommands {
            command: OBDIICommandType::FuelStatus,
            com_string: "FUEL_STATUS".to_string(),
            ret: PythonReturns::PyStr,
        },
    ];

    loop {
        let pyobd_res = py.import("obdii_connect.py");

        match pyobd_res {
            Ok(_) => {
                break;
            }
            Err(_) => {
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        }
    }

    while !thread_info.close.lock().unwrap().get() {
        for command in commands.iter() {
            let locals = PyDict::new(py);
            locals.set_item(py, "s", &command.com_string)?;
            let py_ret = py.eval("c_get_data(s)", None, Some(&locals))?;

            let data: OBDIIData;

            if command.ret == PythonReturns::Float {
                let ret: f64 = py_ret.extract(py)?;

                data = OBDIIData {
                    command: command.command,
                    val: PythonValues { float: ret },
                };
            } else if command.ret == PythonReturns::Long {
                let ret: u32 = py_ret.extract(py)?;

                data = OBDIIData {
                    command: command.command,
                    val: PythonValues { long: ret },
                };
            } else {
                let _ret: String = py_ret.extract(py)?;

                data = OBDIIData {
                    command: command.command,
                    val: PythonValues {
                        fuel_status: OBDIIFuelStatus::OpenLoopTemp,
                    },
                };
            }

            thread_info.obdii_tx.send(data).unwrap();
        }
    }

    Ok(())
}
