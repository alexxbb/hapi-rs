use crate::ffi;
use std::cell::Cell;
use std::ffi::{CString};


#[derive(Debug, Clone)]
pub struct Node {
    pub(crate) id: ffi::HAPI_NodeId,
    pub(crate) inner: Option<Cell<ffi::HAPI_NodeInfo>>,
    pub(crate) session: *const ffi::HAPI_Session
}


// pub fn HAPI_GetNodeInfo(
//     session: *const HAPI_Session,
//     node_id: HAPI_NodeId,
//     node_info: *mut HAPI_NodeInfo,
// ) -> HAPI_Result;

impl Node {
    pub fn create(name: &str,
                  label: &str,
                  session: *const ffi::HAPI_Session,
                  cook: bool,
                  parent: Option<Node>) {
        let name = CString::new(name);
        // let r = ffi::HAPI_CreateNode(session,
        //                              parent.map(|n|n.id).unwrap_or(-1),
        // );

    }
    pub fn info(&self) {

    }
}