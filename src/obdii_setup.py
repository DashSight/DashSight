import sys
import obd
from obd import OBDStatus

obd.logger.setLevel(obd.logging.DEBUG)

main_list = []

class LapTimerOBD(object):
	def __init__(self):
		self.connection = obd.OBD("/dev/ttyS1", baudrate=9600, protocol="3", fast=False)

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
		print("2")
		if self.connection.status() != OBDStatus.CAR_CONNECTED:
			print("No connection to car")
			return 0

		ret = self.coms[self.cur_pos]
		self.cur_pos = self.cur_pos + 1
		if self.cur_pos > self.coms.count:
			self.cur_pos = 0

def c_get_data():
	print("1")
	return main_list[0].get_data()

def c_main():
	main_list.append(LapTimerOBD())
