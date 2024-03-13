import hapi
import json

session = hapi.createInProcessSession()

cook_options = hapi.CookOptions()
try:
    hapi.initialize(session, cook_options)
except hapi.AlreadyInitializedError:
    pass


def get_string(handle) -> str:
    _len = hapi.getStringBufLength(session, handle)
    return hapi.getString(session, handle, _len)


# Create a node
node = hapi.createInputNode(session, "input_node")

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

# Create Dictionary Attribute
attr_info = hapi.AttributeInfo(
    exists=True,
    owner=hapi.attributeOwner.Detail,
    count=1,
    tupleSize=1,
    storage=hapi.storageType.Dictionary,
    originalOwner=hapi.attributeOwner.Detail,
    totalArrayElements=0,  # THIS MUST BE 0 FOR THIS EXAMPLE
)

# Uncomment to see getAttributeDictionaryData fail with error (Houdini 20.0.625):
#   "hapi.InvalidArgumentError: Invalid argument given: Data array length must match AttributeInfo.totalArrayElements."
# ================================
# attr_info.totalArrayElements = 1
# ================================

# Uncomment to see Hython segmentation fault
# =========================
# attr_info.tupleSize = 7
# =========================

DICT_ATTR = "my_dict_attr"

hapi.addAttribute(session, node, 0, DICT_ATTR, attr_info)

in_data = {"foo": 7}

hapi.setAttributeDictionaryData(
    session,
    node,
    0,
    DICT_ATTR,
    attr_info,
    [json.dumps(in_data)],  # data
    0,  # start
    1,  # sizes length
)

hapi.commitGeo(session, node)
hapi.cookNode(session, node, hapi.CookOptions())

handles = hapi.getAttributeDictionaryData(session, node, 0, DICT_ATTR, attr_info, 0, 1)
out_json = get_string(handles[0])
assert in_data == json.loads(out_json)
