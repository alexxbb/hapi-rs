use crate::auto::bindings::{HAPI_AttributeInfo_Create, HAPI_AddAttribute};
use crate::auto::rusty::*;
use std::fs::write;

/*
HAPI_AttributeInfo pos_attr_info;
pos_attr_info.exists = true;
pos_attr_info.owner = HAPI_ATTROWNER_POINT;
pos_attr_info.storage = HAPI_STORAGETYPE_FLOAT;
pos_attr_info.count = meshFn.numVertices();
pos_attr_info.tupleSize = 3;
HAPI_AddAttribute(
    nullptr,
    myInputAssetId,
    myInputObjectId,
    myInputGeoId,
    "P",
    &pos_attr_info );
HAPI_SetAttributeFloatData(
    nullptr,
    myInputAssetId,
    myInputObjectId,
    myInputGeoId,
    "P",
    &pos_attr_info,
    meshFn.getRawPoints( NULL ),
    0,
    meshFn.numVertices() );
 */

pub struct HAPI_AttributeInfo {
    // pub exists: HAPI_Bool,
// pub owner: HAPI_AttributeOwner,
// pub storage: HAPI_StorageType,
// pub originalOwner: HAPI_AttributeOwner,
// pub count: ::std::os::raw::c_int,
// pub tupleSize: ::std::os::raw::c_int,
// pub totalArrayElements: HAPI_Int64,
// pub typeInfo: HAPI_AttributeTypeInfo,
}

struct AttribInfo(HAPI_AttributeInfo);

macro_rules! storage {
    ($name:ident, $var:ident) => {
    fn $name(mut self) -> Self {
        self.0.storage = StorageType::$var.into();
    }
    };
}

macro_rules! owner {
    ($name:ident, $var:ident) => {
    fn $name() -> Self {
        let mut ffi = HAPI_AttributeInfo_Create();
        ffi.originalOwner = AttributeOwner::$var.into();
        Self(ffi)
    }
    };
}

impl AttribInfo {
    owner!(point, Point);
    owner!(primitive, Prim);
    owner!(vertex, Vertex);
    owner!(detail, Detail);
    storage!(float, Float);
    storage!(float64, Float);
    storage!(int, Int);
    storage!(int64, Int64);

    fn storage(mut self, s: StorageType) -> Self {
        self.0.storage = s.into();
        self
    }

    fn type_info(mut self, t: AttributeTypeInfo) -> Self {
        self.0.typeInfo = t.into();
        self
    }
    fn count(mut self, c: u32) -> Self {
        self.0.count = c;
        self }
}

fn add_attr_design() {
    let node: Node;
    let attr = node.add_attribute("pscale", AttribInfo::point().float64());
}
