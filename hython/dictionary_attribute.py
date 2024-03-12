import hou
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
    totalArrayElements=0,
)

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

part_info = hapi.getPartInfo(session, node, 0)
num_detail_attributes = part_info.attributeCounts[3]
assert num_detail_attributes, "No detail attributes"


# attr_info = hapi.getAttributeInfo(
#     session, node, 0, DICT_ATTR, hapi.attributeOwner.Detail
# )
# print(attr_info)


# From the docs: sanity check count. Must be equal to the appropriate attribute owner type count in hapi.PartInfo.
for handle in hapi.getAttributeNames(
    session, node, 0, hapi.attributeOwner.Detail, num_detail_attributes
):
    attr_name = get_string(handle)
    if attr_name == DICT_ATTR:
        break
else:
    print(f"{DICT_ATTR} not found")
    exit(1)

val = hapi.getAttributeDictionaryData(session, node, 0, DICT_ATTR, attr_info, 0, 1)
out_json = get_string(val[0])
assert in_data == json.loads(out_json)
