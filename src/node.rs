use crate::errors::{HAPI_Error, Kind, Result};
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
                    Ok(NodeInfo { id, node: self })
                }
                e => hapi_err!(e, self.ffi_session),
            }
        }
    }
}

pub struct NodeInfo<'a> {
    pub(crate) id: ffi::HAPI_NodeInfo,
    pub node: &'a Node,
}

impl NodeInfo<'_> {
    pub fn node_name(&self) -> Result<String> {
        get_string(self.id.nameSH, self.node.ffi_session)
    }

    pub fn node_path(&self) -> Result<String> {
        get_string(self.id.internalNodePathSH, self.node.ffi_session)
    }

    pub fn node_type(&self) -> ffi::HAPI_NodeType {
        self.id.type_
    }
}
