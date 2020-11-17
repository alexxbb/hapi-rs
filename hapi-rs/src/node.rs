use super::errors::*;
use crate::auto::bindings as ffi;
pub use crate::auto::rusty::NodeType;
use crate::session::SessionHandle;
use crate::char_ptr;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::ffi::CString;
use std::ptr::null;

#[derive(Debug)]
#[non_exhaustive]
pub enum HoudiniNode {
    SopNode(SopNode),
    ObjNode(ObjNode),
}

impl HoudiniNode {
    pub fn delete(self) -> Result<()> {
        use HoudiniNode::*;
        let (id, session) = match &self {
            SopNode(n) => (n.id, n.session.ffi_ptr()),
            ObjNode(n) => (n.id, n.session.ffi_ptr()),
        };
        unsafe {
            let mut info = MaybeUninit::uninit();
            ffi::HAPI_GetNodeInfo(session, id, info.as_mut_ptr()).result(session)?;
            let info = info.assume_init();
            // if info.createdPostAssetLoad != 0 {
            //     unimplemented!()
            // }
            ffi::HAPI_DeleteNode(session, id).result(session)
        }
    }

    #[inline]
    fn ffi_id(&self) -> ffi::HAPI_NodeId {
        match &self {
            HoudiniNode::SopNode(n) => n.id,
            HoudiniNode::ObjNode(n) => n.id,
        }
    }

    pub fn create_sync<T: Into<Vec<u8>>>(
        name: T,
        label: Option<T>,
        parent: Option<HoudiniNode>,
        session: Arc<SessionHandle>,
        cook: bool,
    ) -> Result<HoudiniNode> {
        let mut id = MaybeUninit::uninit();
        let parent = parent.map_or(-1, |n| n.ffi_id());
        let mut label_ptr: *const std::os::raw::c_char = null();
        unsafe {
            let mut tmp;
            if let Some(lb) = label {
                tmp = CString::from_vec_unchecked(lb.into());
                label_ptr = tmp.as_ptr();
            }
            let name = CString::from_vec_unchecked(name.into());
            ffi::HAPI_CreateNode(
                session.ffi_ptr(),
                parent,
                name.as_ptr(),
                label_ptr,
                cook as i8,
                id.as_mut_ptr(),
            ).result(session.ffi_ptr())?;
            Ok(HoudiniNode::ObjNode(ObjNode{
                id: id.assume_init(),
                session: Arc::clone(&session)
            }))
        }

    }
}

#[derive(Debug)]
pub struct SopNode {
    id: ffi::HAPI_NodeId,
    session: Arc<SessionHandle>,
}
#[derive(Debug)]
pub struct ObjNode {
    id: ffi::HAPI_NodeId,
    session: Arc<SessionHandle>,
}

impl SopNode {
    fn sop_method(&self) {
        println!("I'm a sop node")
    }
}

impl ObjNode {
    fn obj_method(&self) {
        println!("I'm an obj node")
    }
}
