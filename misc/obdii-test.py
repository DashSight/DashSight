import sys
import obd

connection = obd.OBD("/dev/ttyS0")

cmd = obd.commands.SPEED

response = connection.query(cmd)

print(response.value)

