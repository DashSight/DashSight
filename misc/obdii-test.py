import sys
import obd

obd.logger.setLevel(obd.logging.DEBUG)

connection = obd.OBD("/dev/ttyS0")

cmd = obd.commands.RPM

response = connection.query(cmd)

print(response.value)

