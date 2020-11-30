use super::node::HoudiniNode;
use crate::{errors::Result, ffi::*, stringhandle};
use std::fmt::Formatter;

pub struct NodeInfo<'a> {
    pub(crate) ffi: HAPI_NodeInfo,
    pub node: &'a HoudiniNode,
    pub node_type: i32,
    pub is_valid: bool,
    pub total_cook_count: i32,
    pub unique_houdini_node_id: i32,
    pub parm_count: i32,
    pub parm_int_value_count: i32,
    pub parm_float_value_count: i32,
    pub parm_string_value_count: i32,
    pub parm_choice_count: i32,
    pub child_node_count: i32,
    pub input_count: i32,
    pub output_count: i32,
    pub created_post_asset_load: bool,
    pub is_time_dependent: bool,
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

impl std::fmt::Debug for NodeInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeInfo")
            .field("name", &self.node_name().unwrap())
            .field("type", &node_type_name(self.node_type))
            .field("is_valid", &self.is_valid)
            .field("total_cook_count", &self.total_cook_count)
            .field("parm_count", &self.parm_count)
            .field("time_dependent", &self.is_time_dependent)
            .field("input_count", &self.input_count)
            .field("output_count", &self.output_count)
            .finish()
    }
}

impl<'a> NodeInfo<'a> {
    pub(crate) fn from_ffi(info: HAPI_NodeInfo, node: &'a HoudiniNode) -> Self {
        NodeInfo {
            ffi: info,
            node,
            node_type: info.type_,
            is_valid: info.isValid == 1,
            total_cook_count: info.totalCookCount,
            unique_houdini_node_id: info.uniqueHoudiniNodeId,
            parm_count: info.parmCount,
            parm_int_value_count: info.parmIntValueCount,
            parm_float_value_count: info.parmFloatValueCount,
            parm_string_value_count: info.parmStringValueCount,
            parm_choice_count: info.parmChoiceCount,
            child_node_count: info.childNodeCount,
            input_count: info.inputCount,
            output_count: info.outputCount,
            created_post_asset_load: info.createdPostAssetLoad == 1,
            is_time_dependent: info.isTimeDependent == 1,
        }
    }
    pub fn node_name(&self) -> Result<String> {
        stringhandle::get_string(self.ffi.nameSH, &self.node.session)
    }

    pub fn internal_path(&self) -> Result<String> {
        stringhandle::get_string(self.ffi.internalNodePathSH, &self.node.session)
    }
    pub fn parent_node(&self) -> Option<HoudiniNode> {
        if self.ffi.parentId == -1 {
            None
        } else {
            Some(HoudiniNode {
                id: self.ffi.parentId,
                session: self.node.session.clone(),
            })
        }
    }
}
