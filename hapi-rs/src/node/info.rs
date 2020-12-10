use crate::auto::bindings::NodeType;
use crate::{errors::Result, ffi::*, node::NodeHandle, session::Session, stringhandle};
use std::fmt::Formatter;

pub struct NodeInfo {
    pub(crate) inner: HAPI_NodeInfo,
    pub(crate) session: Session,
}

const fn node_type_name(tp: NodeType) -> &'static str {
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
            .field("name", &self.name().unwrap())
            .field("internal_path", &self.internal_path().unwrap())
            .field("type", &node_type_name(self.node_type()))
            .field("is_valid", &self.is_valid())
            .field("time_dependent", &self.is_time_dependent())
            .field("total_cook_count", &self.total_cook_count())
            .field("parm_count", &self.parm_count())
            .field("child_count", &self.child_node_count())
            .field("input_count", &self.input_count())
            .field("output_count", &self.output_count())
            .finish()
    }
}

impl NodeInfo {
    pub fn new(session: crate::session::Session) -> Self {
        unsafe {
            NodeInfo {
                inner: HAPI_NodeInfo_Create(),
                session,
            }
        }
    }

    fn_getter!(node_type, type_, NodeType);
    fn_getter!(is_valid, isValid, bool);
    fn_getter!(unique_node_id, uniqueHoudiniNodeId, i32);
    fn_getter!(total_cook_count, totalCookCount, i32);
    fn_getter!(child_node_count, childNodeCount, i32);
    fn_getter!(parm_count, parmCount, i32);
    fn_getter!(input_count, inputCount, i32);
    fn_getter!(output_count, outputCount, i32);
    fn_getter!(is_time_dependent, isTimeDependent, bool);
    fn_getter!(created_post_asset_load, createdPostAssetLoad, bool);
    fn_getter!(parm_int_value_count, parmIntValueCount, i32);
    fn_getter!(parm_float_value_count, parmFloatValueCount, i32);
    fn_getter!(parm_string_value_count, parmStringValueCount, i32);
    fn_getter!(parm_choice_count, parmChoiceCount, i32);
    fn_getter!(name, nameSH, Result<String>);
    fn_getter!(internal_path, internalNodePathSH, Result<String>);
    fn_getter!(self, node_handle, { NodeHandle(self.inner.id) } => NodeHandle);
    fn_getter!(self, parent_id, { NodeHandle(self.inner.parentId) } => NodeHandle);
}
