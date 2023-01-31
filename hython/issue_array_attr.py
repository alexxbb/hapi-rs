import hou
import hapi

try:
    session = hapi.createThriftSocketSession('localhost', 9090)
except:
    session = hapi.createInProcessSession()

cook_options = hapi.CookOptions()
try:
    hapi.initialize(session, cook_options)
except hapi.AlreadyInitializedError:
    pass


node = hapi.createInputNode(session, "input_node")

part_info = hapi.PartInfo(
    vertexCount = 0,
    faceCount = 0,
    pointCount = 2,
    type = hapi.partType.Mesh
)

hapi.setPartInfo(session, node, 0, part_info)

# Positions

p_info = hapi.AttributeInfo(
    exists = True,
    owner = hapi.attributeOwner.Point,
    count = 2,
    tupleSize = 3,
    storage = hapi.storageType.Float,
    originalOwner = hapi.attributeOwner.Invalid
)

hapi.addAttribute(session, node, 0, "P", p_info)
hapi.setAttributeFloatData(session, node, 0, "P", p_info,
    [-1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 0, 2)

# Int Array
array_info = hapi.AttributeInfo(
    exists = True,
    owner = hapi.attributeOwner.Point,
    count = 2,
    tupleSize = 1,
    storage = hapi.storageType.IntArray,
    originalOwner = hapi.attributeOwner.Invalid
)

hapi.addAttribute(session, node, 0, "my_array", array_info)
hapi.setAttributeIntArrayData(
        session, 
        node, 
        0, 
        "my_array", 
        array_info,
        [1, 2, 3, 4, 5],   # data
        5,              # data length
        [2, 3],         # sizes array
        0,              # start
        2               # sizes length
)

hapi.commitGeo(session, node)