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
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(PartialEq)]
enum PythonReturns {
    Float,
    Long,
    PyStr,
}

#[derive(Clone, Copy)]
enum OBDIIFuelStatus {
    OpenLoopTemp,
    ClosedLoopO2Sense,
    OpenLoopLoad,
    OpenLoopFailure,
    ClosedLoopFault,
}

union PythonValues {
    float: f64,
    long: u32,
    fuel_status: OBDIIFuelStatus,
}

pub struct OBDIICommands {
    command: String,
    ret: PythonReturns,
    val: PythonValues,
}

pub fn obdii_thread(thread_info: ThreadingRef) -> PyResult<()> {
    let gli = Python::acquire_gil();
    let py = gli.python();

    let commands: [OBDIICommands; 10] = [
        OBDIICommands {
            command: "RPM".to_string(),
            ret: PythonReturns::Float,
            val: PythonValues{ float: 0.0 },
        },
        OBDIICommands {
            command: "THROTTLE_POS".to_string(),
            ret: PythonReturns::Float,
            val: PythonValues{ float: 0.0 },
        },
        OBDIICommands {
            command: "ENGINE_LOAD".to_string(),
            ret: PythonReturns::Float,
            val: PythonValues{ float: 0.0 },
        },
        OBDIICommands {
            command: "TIMING_ADVANCE".to_string(),
            ret: PythonReturns::Float,
            val: PythonValues{ float: 0.0 },
        },
        OBDIICommands {
            command: "MAF".to_string(),
            ret: PythonReturns::Float,
            val: PythonValues{ float: 0.0 },
        },
        OBDIICommands {
            command: "COOLANT_TEMP".to_string(),
            ret: PythonReturns::Long,
            val: PythonValues{ long: 0 },
        },
        OBDIICommands {
            command: "INTAKE_TEMP".to_string(),
            ret: PythonReturns::Long,
            val: PythonValues{ long: 0 },
        },
        OBDIICommands {
            command: "SHORT_FUEL_TRIM_1".to_string(),
            ret: PythonReturns::Long,
            val: PythonValues{ long: 0 },
        },
        OBDIICommands {
            command: "LONG_FUEL_TRIM_1".to_string(),
            ret: PythonReturns::Long,
            val: PythonValues{ long: 0 },
        },
        OBDIICommands {
            command: "FUEL_STATUS".to_string(),
            ret: PythonReturns::PyStr,
            val: PythonValues{ fuel_status: OBDIIFuelStatus::OpenLoopTemp },
        },
    ];

    loop {
	    let pyobd_res = py.import("obdii_connect.py");

	    match pyobd_res {
	    	Ok(_) => {
	    		break;
	    	}
	    	Err(_) => {
	    		std::thread::sleep(std::time::Duration::from_secs(5));
	    		continue;
	    	}
	    }
	}

    while !thread_info.close.lock().unwrap().get() {
        for command in commands.iter() {
            let locals = PyDict::new(py);
            locals.set_item(py, "s", &command.command)?;
            let py_ret = py.eval("c_get_data(s)", None, Some(&locals))?;

            let mut com = command.clone();

            if command.ret == PythonReturns::Float {
                let ret: f64 = py_ret.extract(py)?;

                com.val.float = ret;
            } else if command.ret == PythonReturns::Long {
                let ret: u32 = py_ret.extract(py)?;

                com.val.long = ret;
            } else if command.ret == PythonReturns::PyStr {
                let _ret: String = py_ret.extract(py)?;

                // com.val.fuel_status = ret;
            }

            thread_info.obdii_tx.send(com).unwrap();
        }
    }

    Ok(())
}
