import hapi
import json

session = hapi.createInProcessSession(hapi.SessionInfo())
hapi.initialize(session, hapi.CookOptions())


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

    return node


node = _create_input_point()

# Create Dictionary Attribute
attr_info = hapi.AttributeInfo(
    exists=True,
    owner=hapi.attributeOwner.Detail,
    count=1,
    tupleSize=1,
    storage=hapi.storageType.DictionaryArray,
)


data = [json.dumps({"foo": 7}), json.dumps({"bar": 3})]

hapi.addAttribute(session, node, 0, "my_dict_attr", attr_info)

# Uncomment to see HARS segfalt if commiting geo BEFORE setting attribute data
# hapi.commitGeo(session, node)

hapi.setAttributeDictionaryArrayData(
    session,
    node,
    0,
    "my_dict_attr",
    attr_info,
    data,  # data
    2,
    [2],
    0,  # start
    1,  # sizes length
)

hapi.commitGeo(session, node)
hapi.cookNode(session, node, hapi.CookOptions())
