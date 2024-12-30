
import hapi

options = hapi.ThriftServerOptions(autoClose=True)
hapi.startThriftSharedMemoryServer(options, "mem", None)
session = hapi.createThriftSharedMemorySession("mem", hapi.SessionInfo())
hapi.initialize(session, hapi.CookOptions(), use_cooking_thread=True)


def _create_input_point() -> int:
    node = hapi.createInputNode(session, -1, "input_node")

    part_info = hapi.PartInfo(
        vertexCount=0, faceCount=0, pointCount=1, type=hapi.partType.Mesh
    )

    hapi.setPartInfo(session, node, 0, part_info)

    # Positions
    p_info = hapi.AttributeInfo(
        exists=True,
        owner=hapi.attributeOwner.Point,
        count=1,
        tupleSize=3,
        storage=hapi.storageType.Float,
        originalOwner=hapi.attributeOwner.Invalid,
    )

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






