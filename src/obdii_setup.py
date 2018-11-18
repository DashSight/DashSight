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

	def get_data(self, cmd):
		"""
		Get the current OBDII data and increment the command.
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

def c_get_data(com):
	return lap_timer.get_data(com)

lap_timer = LapTimerOBD()

