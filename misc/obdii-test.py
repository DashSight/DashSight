# Copyright 2018 Alistair Francis <alistair@alistair23.me>
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import sys
import time
import obd
from obd import OBDStatus

obd.logger.setLevel(obd.logging.DEBUG)

connection = obd.OBD("/dev/ttyS1", baudrate=9600, protocol="3", fast=False)

if connection.status() != OBDStatus.CAR_CONNECTED:
    print("Unable to connect to the car")
    exit(1)

print("Listing supported commands:")
# print(connection.supported_commands)
for c in connection.supported_commands:
        print(str(c))
        response = connection.query(c)
        print(response.value)

