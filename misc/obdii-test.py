import sys
import obd
from obd import OBDStatus

obd.logger.setLevel(obd.logging.DEBUG)

connection = obd.OBD("/dev/ttyS0")

if connection.status() != OBDStatus.CAR_CONNECTED:
    print "Unable to connect to the car"
    # exit(1)

for pid in range(1, 0x5E):
    cmd = obd.commands.has_pid(1, pid)

    if connection.supports(cmd):
        response = connection.query(cmd)
        print "Car supports command pid: " + str(pid)
        print(response.value)

