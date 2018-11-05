import sys
import obd
from obd import OBDStatus

obd.logger.setLevel(obd.logging.DEBUG)

connection = obd.OBD("/dev/ttyS1", baudrate=9600, protocol="3", fast=False)

if connection.status() != OBDStatus.CAR_CONNECTED:
    print("Unable to connect to the car")
    exit(1)
