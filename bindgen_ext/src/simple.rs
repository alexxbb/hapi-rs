pub type HAPI_NodeId = ::std::os::raw::c_int;
pub type HAPI_StringHandle = ::std::os::raw::c_int;
pub type HAPI_Bool = ::std::os::raw::c_char;
pub enum HAPI_NodeType {
    HAPI_NODETYPE_ANY = -1,
    HAPI_NODETYPE_NONE = 0,
    HAPI_NODETYPE_OBJ = 1,
    HAPI_NODETYPE_SOP = 2,
    HAPI_NODETYPE_CHOP = 4,
    HAPI_NODETYPE_ROP = 8,
    HAPI_NODETYPE_SHOP = 16,
    HAPI_NODETYPE_COP = 32,
    HAPI_NODETYPE_VOP = 64,
    HAPI_NODETYPE_DOP = 128,
    HAPI_NODETYPE_TOP = 256,
}

pub struct HAPI_NodeInfo {
    pub id: HAPI_NodeId,
    pub parentId: HAPI_NodeId,
    pub nameSH: HAPI_StringHandle,
    pub type_: HAPI_NodeType,
    pub isValid: HAPI_Bool,
    pub totalCookCount: ::std::os::raw::c_int,
    pub uniqueHoudiniNodeId: ::std::os::raw::c_int,
    pub internalNodePathSH: HAPI_StringHandle,
    pub parmCount: ::std::os::raw::c_int,
    pub parmIntValueCount: ::std::os::raw::c_int,
    pub parmFloatValueCount: ::std::os::raw::c_int,
    pub parmStringValueCount: ::std::os::raw::c_int,
    pub parmChoiceCount: ::std::os::raw::c_int,
    pub childNodeCount: ::std::os::raw::c_int,
    pub inputCount: ::std::os::raw::c_int,
    pub outputCount: ::std::os::raw::c_int,
    pub createdPostAssetLoad: HAPI_Bool,
    pub isTimeDependent: HAPI_Bool,
}
