import hapi

session = hapi.createInProcessSession(hapi.SessionInfo())
hapi.initialize(session, hapi.CookOptions(), True)

hda_bytes = open('otls/hapi_parms.hda', 'rb').read()
lib_id = hapi.loadAssetLibraryFromMemory(session, hda_bytes, len(hda_bytes), True)

asset = hapi.createNode(session, -1, "Objects/hapi_parms", "", True)

info = hapi.getNodeInfo(session, asset)

parms = hapi.getParameters(session, asset, 0, info.parmCount)

parm_info = hapi.getParmInfo(session, asset, parms[0].id)
print(parm_info)

