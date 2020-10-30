use crate::ffi;
use crate::errors::{Result, HAPI_Error, Kind};
use std::cell::Cell;
use std::ffi::{CString};
use std::mem::MaybeUninit;


#[derive(Debug, Clone)]
pub struct Node {
    pub(crate) id: ffi::HAPI_NodeId,
    pub(crate) inner: Option<Cell<ffi::HAPI_NodeInfo>>,
    pub(crate) session: *const ffi::HAPI_Session,
}

impl Node {
    pub fn create(name: &str,
                  label: &str,
                  session: *const ffi::HAPI_Session,
                  cook: bool,
                  parent: Option<Node>) -> Result<Node> {
        let name = CString::new(name)?;
        let label = CString::new(label)?;
        // TODO: Make sure to check if name contains the table name
        let mut id = MaybeUninit::uninit();
        unsafe {
            match ffi::HAPI_CreateNode(session,
                                       parent.map(|n| n.id).unwrap_or(-1),
                                       name.as_ptr(),
                                       label.as_ptr(),
                                       cook as i8,
                                       id.as_mut_ptr()) {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => {
                    let id = id.assume_init();
                    Ok(Node { id, inner: None, session })
                }

                e => Err(HAPI_Error::new(Kind::Hapi(e), Some(session)))
            }
        }
    }
    pub fn info(&self) {}
}