use crate::errors::{HapiError, Kind, Result};
use crate::ffi;
use crate::hapi_err;
use crate::stringhandle::get_string;
use std::ffi::CString;
use std::mem::MaybeUninit;

#[derive(Debug, Clone)]
pub struct Node {
    pub(crate) ffi_id: ffi::HAPI_NodeId,
    pub(crate) ffi_session: *const ffi::HAPI_Session,
}

impl Node {
    pub fn create(
        name: &str,
        label: &str,
        session: *const ffi::HAPI_Session,
        cook: bool,
        parent: Option<Node>,
    ) -> Result<Node> {
        let name = CString::new(name)?;
        let label = CString::new(label)?;
        // TODO: Make sure to check if name contains the table name
        let mut id = MaybeUninit::uninit();
        unsafe {
            match ffi::HAPI_CreateNode(
                session,
                parent.map(|n| n.ffi_id).unwrap_or(-1),
                name.as_ptr(),
                label.as_ptr(),
                cook as i8,
                id.as_mut_ptr(),
            ) {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => {
                    let id = id.assume_init();
                    Ok(Node {
                        ffi_id: id,
                        ffi_session: session,
                    })
                }

                e => hapi_err!(e, session),
            }
        }
    }
    pub fn info(&self) -> Result<NodeInfo<'_>> {
        let mut id = MaybeUninit::uninit();
        unsafe {
            let r = ffi::HAPI_GetNodeInfo(self.ffi_session, self.ffi_id, id.as_mut_ptr());
            match r {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => {
                    let id = id.assume_init();
                    Ok(NodeInfo { inner: id, node: self })
                }
                e => hapi_err!(e, self.ffi_session),
            }
        }
    }
}

#[derive(Debug)]
pub struct NodeInfo<'a> {
    pub(crate) inner: ffi::HAPI_NodeInfo,
    pub node: &'a Node,
}

impl NodeInfo<'_> {
    _inner_filed!(nameSH, node_name, node.ffi_session, Result<String>);
    _inner_filed!(internalNodePathSH, node_path, node.ffi_session, Result<String>);
    _inner_filed!(type_, node_type, ffi::HAPI_NodeType);
    _inner_filed!(isValid, is_valid, bool);
    _inner_filed!(parmCount, parm_count, i32);
    _inner_filed!(totalCookCount, total_cook_count, i32);
    _inner_filed!(uniqueHoudiniNodeId, unique_node_id, i32);
    _inner_filed!(parmIntValueCount, parm_int_value_count, i32);
    _inner_filed!(parmFloatValueCount, parm_flt_value_count, i32);
    _inner_filed!(parmStringValueCount, parm_str_value_count, i32);
    _inner_filed!(parmChoiceCount, parm_choice_count, i32);
    _inner_filed!(childNodeCount, child_node_count, i32);
    _inner_filed!(inputCount, input_count, i32);
    _inner_filed!(outputCount, output_count, i32);
    _inner_filed!(createdPostAssetLoad, create_post_asset_load, bool);
    _inner_filed!(isTimeDependent, is_time_dependent, bool);
}