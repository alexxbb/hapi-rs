import hapi
import os
import random



def start_server() -> hapi.license:
    options = hapi.ThriftServerOptions()
    options.autoClose = True
    connection_name = f"hapi-{random.randint(0, 100)}"
    hapi.startThriftSharedMemoryServer(options, connection_name, "")
    session = hapi.createThriftSharedMemorySession(connection_name, hapi.SessionInfo())
    hapi.initialize(session, hapi.CookOptions(), use_cooking_thread=True)

    # Creating node triggers license check
    hapi.createNode(session, -1, "Object/null", "", True)
    license_type = hapi.license(hapi.getSessionEnvInt(session, hapi.sessionEnvIntType.License)) 
    return license_type


os.environ["HOUDINI_PLUGIN_LIC_OPT"] = "--skip-licenses=Houdini-Engine"
license_type = start_server()
print("License type: ", license_type)
assert license_type == hapi.license.Houdini
print("--------------------------------")

os.environ["HOUDINI_PLUGIN_LIC_OPT"] = "--check-licenses=Houdini-Engine --skip-licenses=Houdini-Escape"
license_type = start_server()
print("License type: ", license_type)
assert license_type == hapi.license.HoudiniEngine
print("--------------------------------")

os.environ["HOUDINI_PLUGIN_LIC_OPT"] = "--check-licenses=Houdini-Escape --skip-licenses=Houdini-Engine"
license_type = start_server()
print("License type: ", license_type)
assert license_type == hapi.license.Houdini
