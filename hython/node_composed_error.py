import hapi

session = hapi.createInProcessSession(hapi.SessionInfo())
hapi.initialize(session, hapi.CookOptions())

hda_bytes = open('otls/hapi_errors.hda', 'rb').read()
lib_id = hapi.loadAssetLibraryFromMemory(session, hda_bytes, len(hda_bytes), True)

asset = hapi.createNode(session, -1, "Object/hapi_rs::errors")
res = hapi.cookNode(session, asset, hapi.CookOptions())

# # cook geo
geo = hapi.getDisplayGeoInfo(session, asset)
res = hapi.cookNode(session, geo.nodeId, hapi.CookOptions())

# Retrieve errors
length = hapi.composeNodeCookResult(session, asset, hapi.statusVerbosity._2)
res = hapi.getComposedNodeCookResult(session, length)
print(res)
