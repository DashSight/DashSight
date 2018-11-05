print("Listing supported commands:")
# print(connection.supported_commands)
for c in connection.supported_commands:
        print(str(c))
        response = connection.query(c)
        print(response.value)
