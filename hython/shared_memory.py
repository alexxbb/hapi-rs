
import hapi
import random

server_options = hapi.ThriftServerOptions()
server_options.autoClose = False

connection_name = "hars-mem{}".format(random.randint(0, 1000000))
server_options.sharedMemoryBufferType = hapi.thriftSharedMemoryBufferType.Buffer
server_options.sharedMemoryBufferSize = 1024 * 1024 * 10
hapi.startThriftSharedMemoryServer(server_options, connection_name, "/tmp/foo.log")

session_info = hapi.SessionInfo()
# CONNECTION COUNT > 0 is crashing the server
session_info.connectionCount = 0
session_info.sharedMemoryBufferType = server_options.sharedMemoryBufferType
session_info.sharedMemoryBufferSize = server_options.sharedMemoryBufferSize
session = hapi.createThriftSharedMemorySession(connection_name, session_info)
hapi.initialize(session, hapi.CookOptions(), use_cooking_thread=True)


print(session_info.connectionCount)



