import sys
import obd
from obd import OBDStatus

obd.logger.setLevel(obd.logging.DEBUG)

connection = obd.OBD("/dev/ttyS0", baudrate=9600, protocol=3, fast=False)

if connection.status() != OBDStatus.CAR_CONNECTED:
    print "Unable to connect to the car"
    exit(1)

print "Listing supported commands:"
for c in obd.supported_commands:
	print(str(c))

print ""

for pid in range(1, 0x20):
    cmd = obd.commands[1, pid]

    if connection.supports(cmd):
        response = connection.query(cmd)
        print "Car supports command pid: " + str(pid)
        print(response.message)
        print(response.value)

