use crate::{
    ffi,
    ffi::raw::{NodeType, HAPI_NodeInfo},
    errors::Result, node::NodeHandle, session::Session, stringhandle
};
use std::fmt::Formatter;
use std::mem::MaybeUninit;

#[derive(Clone)]
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
    pub fn new(session: Session, node: &NodeHandle) -> Result<Self> {
        let inner = unsafe {
            let mut inner = MaybeUninit::uninit();
            ffi::raw::HAPI_GetNodeInfo(session.ptr(), node.0, inner.as_mut_ptr())
                .result_with_session(|| session.clone())?;
            inner.assume_init()
        };
        Ok(NodeInfo { inner, session })
    }

    pub fn name(&self) -> Result<String> {
        self.session.get_string(self.inner.nameSH)
    }
    pub fn internal_path(&self) -> Result<String> {
        self.session.get_string(self.inner.internalNodePathSH)
    }

    get!(node_type->type_->NodeType);
    get!(is_valid->isValid->bool);
    get!(unique_node_id->uniqueHoudiniNodeId->i32);
    get!(total_cook_count->totalCookCount->i32);
    get!(child_node_count->childNodeCount->i32);
    get!(parm_count->parmCount->i32);
    get!(input_count->inputCount->i32);
    get!(output_count->outputCount->i32);
    get!(is_time_dependent->isTimeDependent->bool);
    get!(created_post_asset_load->createdPostAssetLoad->bool);
    get!(parm_int_value_count->parmIntValueCount->i32);
    get!(parm_float_value_count->parmFloatValueCount->i32);
    get!(parm_string_value_count->parmStringValueCount->i32);
    get!(parm_choice_count->parmChoiceCount->i32);
    get!(node_handle->id->[handle: NodeHandle]);
    get!(parent_id->parentId->[handle: NodeHandle]);
}
