use crate::inner_field;
use crate::{errors::Result, ffi::*, node::NodeHandle, session::Session, stringhandle};
use std::fmt::Formatter;

pub struct NodeInfo {
    pub(crate) inner: HAPI_NodeInfo,
    pub(crate) session: Session,
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

    inner_field!(type_, node_type, i32);
    inner_field!(isValid, is_valid, bool);
    inner_field!(uniqueHoudiniNodeId, unique_node_id, i32);
    inner_field!(totalCookCount, total_cook_count, i32);
    inner_field!(childNodeCount, child_node_count, i32);
    inner_field!(parmCount, parm_count, i32);
    inner_field!(inputCount, input_count, i32);
    inner_field!(outputCount, output_count, i32);
    inner_field!(isTimeDependent, is_time_dependent, bool);
    inner_field!(createdPostAssetLoad, created_post_asset_load, bool);
    inner_field!(parmIntValueCount, parm_int_value_count, i32);
    inner_field!(parmFloatValueCount, parm_float_value_count, i32);
    inner_field!(parmStringValueCount, parm_string_value_count, i32);
    inner_field!(parmChoiceCount, parm_choice_count, i32);
    inner_field!(nameSH, name, Result<String>);
    inner_field!(internalNodePathSH, internal_path, Result<String>);

    #[inline]
    pub fn node_handle(&self) -> NodeHandle {
        NodeHandle(self.inner.id)
    }

    #[inline]
    pub fn parent_id(&self) -> NodeHandle {
        NodeHandle(self.inner.parentId)
    }
}
