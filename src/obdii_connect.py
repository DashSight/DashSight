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
import obd
from obd import OBDStatus

# obd.logger.setLevel(obd.logging.DEBUG)

class LapTimerOBD(object):
	def __init__(self):
		self.connection = obd.OBD("/dev/ttyS3", start_low_power=True)

		if self.connection.status() != OBDStatus.CAR_CONNECTED:
			print("Unable to connect to the car")
			exit(1)

	def get_data(self, cmd):
		"""
			Get the current OBDII data and increment the command.

			Returns -1 on failure. On success the fail returned by the
			car is returned.
		"""
		if self.connection.status() != OBDStatus.CAR_CONNECTED:
			print("No connection to car")
			return -1

		ret = self.connection.query(obd.commands[cmd])

		try:
			ret = ret.value.magnitude
		except:
			ret = ret.value

		return ret

	def enter_low_power(self):
		"""
			Enter low power mode

			Returns 0 on success and -1 on failure.
		"""
		line = self.connection.low_power()

		if 'OK' in lines:
			return 0

		return -1

def c_get_data(com):
	return lap_timer.get_data(com)

def c_enter_low_power(com):
	return lap_timer.enter_low_power()

lap_timer = LapTimerOBD()

