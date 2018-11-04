import sys
import time
import obd
from obd import OBDStatus

obd.logger.setLevel(obd.logging.DEBUG)

connection = obd.OBD("/dev/ttyS2", baudrate=9600, protocol="3", fast=False)

if connection.status() != OBDStatus.CAR_CONNECTED:
    print("Unable to connect to the car")
    exit(1)

print("Listing supported commands:")
print(connection.supported_commands)
for c in connection.supported_commands:
        print(str(c))
        response = connection.query(c)
        print(response.value)

#for pid in range(1, 0x20):
#    time.sleep(3)
#    cmd = obd.commands[1, pid]
#
#    response = connection.query(pid)
#    print("Car supports command pid: " + str(pid))
#    print(response.value)

