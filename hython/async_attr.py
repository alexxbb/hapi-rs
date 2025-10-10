
import hapi

options = hapi.ThriftServerOptions()
options.autoClose = True
hapi.startThriftSharedMemoryServer(options, "mem", "")
session = hapi.createThriftSharedMemorySession("mem", hapi.SessionInfo())
hapi.initialize(session, hapi.CookOptions(), use_cooking_thread=True)


def _create_input_point() -> int:
    node = hapi.createInputNode(session, -1, "input_node")

    part_info = hapi.PartInfo()
    part_info.vertexCount = 0
    part_info.faceCount = 0
    part_info.pointCount = 1
    part_info.type = hapi.partType.Mesh

    hapi.setPartInfo(session, node, 0, part_info)

    # Positions
    p_info = hapi.AttributeInfo()
    p_info.exists = True
    p_info.owner = hapi.attributeOwner.Point
    p_info.count = 1
    p_info.tupleSize = 3
    p_info.storage = hapi.storageType.Float
    p_info.originalOwner = hapi.attributeOwner.Invalid
    
    hapi.addAttribute(session, node, 0, "P", p_info)
    hapi.setAttributeFloatData(session, node, 0, "P", p_info, [0.0, 0.0, 0.0], 0, 1)

    return p_info, node

info, node = _create_input_point()
hapi.commitGeo(session, node)
hapi.cookNode(session, node, None)
while not hapi.getStatus(session, hapi.statusType.CookState) == hapi.state.Ready:
    ...

# This is working
# data = hapi.getAttributeFloatData(session, node, 0, "P", info, -1, 0, info.count)

# This is not
data, job_id = hapi.getAttributeFloatDataAsync(session, node, 0, "P", info, -1, 0, info.count)
status = hapi.getJobStatus(session, job_id)






