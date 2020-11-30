use crate::{errors::Result, ffi::*, stringhandle};
use std::fmt::Formatter;

pub struct NodeInfo {
    pub(crate) inner: HAPI_NodeInfo,
}

impl Default for NodeInfo {
    fn default() -> Self {
        unsafe {
            NodeInfo{inner: HAPI_NodeInfo_Create()}
        }
    }
}

const fn node_type_name(tp: i32) -> &'static str {
    use crate::auto::rusty::NodeType;
    match tp {
        NodeType::Sop => "Sop",
        NodeType::Obj => "Obj",
        NodeType::Rop => "Rop",
        NodeType::Dop => "Dop",
        NodeType::Cop => "Cop",
        NodeType::Shop => "Shop",
        NodeType::Vop => "Vop",
        NodeType::Chop => "Chop",
        _ => "Unknown",
    }
}

impl std::fmt::Debug for NodeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeInfo")
            // .field("name", &self.node_name().unwrap())
            // .field("type", &node_type_name(self.node_type))
            // .field("is_valid", &self.is_valid)
            // .field("total_cook_count", &self.total_cook_count)
            // .field("parm_count", &self.parm_count)
            // .field("time_dependent", &self.is_time_dependent)
            // .field("input_count", &self.input_count)
            // .field("output_count", &self.output_count)
            .finish()
    }
}

impl NodeInfo {
    pub(crate) fn from_ffi(info: HAPI_NodeInfo) -> Self {
        NodeInfo { inner: info }
    }

    pub fn is_valid(&self) -> bool {
        self.inner.isValid == 1
    }

    pub fn unique_houdini_node_id(&self) -> i32 {
        self.inner.uniqueHoudiniNodeId
    }

    // TODO implement methods
    // pub node_type: i32,
    // pub is_valid: bool,
    // pub total_cook_count: i32,
    // pub parm_count: i32,
    // pub parm_int_value_count: i32,
    // pub parm_float_value_count: i32,
    // pub parm_string_value_count: i32,
    // pub parm_choice_count: i32,
    // pub child_node_count: i32,
    // pub input_count: i32,
    // pub output_count: i32,
    // pub created_post_asset_load: bool,
    // pub is_time_dependent: bool,

    // pub fn node_name(&self) -> Result<String> {
    //     stringhandle::get_string(self.name_sh, &self.node.session)
    // }
    //
    // pub fn internal_path(&self) -> Result<String> {
    //     stringhandle::get_string(self.internal_node_sh, &self.node.session)
    // }
}
