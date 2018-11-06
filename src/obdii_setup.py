import sys
import obd
from obd import OBDStatus

# obd.logger.setLevel(obd.logging.DEBUG)

class LapTimerOBD(object):
	def __init__(self):
		self.connection = obd.OBD("/dev/ttyS1", fast=False)

		if self.connection.status() != OBDStatus.CAR_CONNECTED:
			print("Unable to connect to the car")
			exit(1)

		self.get_working_commands()

	def get_working_commands(self):
		"""
		Query the car and store all commands that we support and return data.
		"""

		# Setup the data we will need later.
		self.coms = []
		self.cur_pos = 0

		for c in self.connection.supported_commands:
			response = self.connection.query(c)
			if not response.is_null():
				self.coms.append(c)
				
	def get_data(self):
		if self.connection.status() != OBDStatus.CAR_CONNECTED:
			print("No connection to car")
			return -1

		ret = self.coms[self.cur_pos]
		self.cur_pos = self.cur_pos + 1
		if self.cur_pos > len(self.coms) - 1:
			self.cur_pos = 0

		ret = self.connection.query(ret)

		return ret.value

def c_get_data():
	return lap_timer.get_data()

lap_timer = LapTimerOBD()

