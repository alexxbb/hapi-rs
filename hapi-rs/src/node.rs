use crate::{
    auto::bindings as ffi,
    auto::rusty::{
        NodeType,
        State,
        StatusType,
        StatusVerbosity
    },
    cookoptions::CookOptions,
    errors::*,
    session::{Session, CookResult},
};
use std::{
    ffi::CString,
    mem::MaybeUninit,
    pin::Pin,
    ptr::null,
    sync::Arc,
    task::{Context, Poll},
};

use log::{debug};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum HoudiniNode {
    SopNode(SopNode),
    ObjNode(ObjNode),
}

impl HoudiniNode {
    pub fn delete(self) -> Result<()> {
        use HoudiniNode::*;
        let (id, session) = self.strip();
        unsafe {
            let mut info = MaybeUninit::uninit();
            ffi::HAPI_GetNodeInfo(session.ptr(), id, info.as_mut_ptr())
                .result_with_session(|| session.clone())?;
            let info = info.assume_init();
            // if info.createdPostAssetLoad != 0 {
            //     unimplemented!()
            // }
            ffi::HAPI_DeleteNode(session.ptr(), id).result_with_session(|| session.clone())
        }
    }

    #[inline]
    fn strip(&self) -> (ffi::HAPI_NodeId, &Session) {
        match &self {
            HoudiniNode::SopNode(n) => (n.id, &n.session),
            HoudiniNode::ObjNode(n) => (n.id, &n.session),
        }
    }

    /// https://github.com/sideeffects/HoudiniEngineForUnity/blob/5b2d34bd5a04513288f4991048bf9c5ecceacac5/Plugins/HoudiniEngineUnity/Scripts/Asset/HEU_HoudiniAsset.cs#L1536
    pub fn cook(&self, options: Option<CookOptions>) -> Result<()> {
        let (id, session) = self.strip();
        let opt = options.map(|o| o.ptr()).unwrap_or(null());
        unsafe {
            ffi::HAPI_CookNode(session.ptr(), id, opt).result_with_session(|| session.clone())?;
        }
        Ok(())
    }

    pub fn cook_blocking(&self, options: Option<CookOptions>) -> Result<CookResult> {
        self.cook(options)?;
        let (_, session) = self.strip();
        session.cook_result()
    }


    fn _create(
        name: &str,
        label: Option<&str>,
        parent: Option<HoudiniNode>,
        session: &Session,
        cook: bool,
    ) -> Result<i32> {
        let mut id = MaybeUninit::uninit();
        let parent = parent.map_or(-1, |n| n.strip().0);
        let mut label_ptr: *const std::os::raw::c_char = null();
        unsafe {
            let mut tmp;
            if let Some(lb) = label {
                tmp = CString::from_vec_unchecked(lb.into());
                label_ptr = tmp.as_ptr();
            }
            debug!("Creating node: {}", name);
            let name = CString::from_vec_unchecked(name.into());
            ffi::HAPI_CreateNode(
                session.ptr(),
                parent,
                name.as_ptr(),
                label_ptr,
                cook as i8,
                id.as_mut_ptr(),
            )
            .result_with_session(|| session.clone())?;
            Ok(id.assume_init())
        }
    }

    pub fn create_blocking(
        name: &str,
        label: Option<&str>,
        parent: Option<HoudiniNode>,
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        let id = HoudiniNode::_create(name, label, parent, &session, cook)?;
        if session.unsync {
            loop {
                match session.get_status(StatusType::CookState)? {
                    State::Ready => break,
                    _ => {}
                }
            }
        }
        Ok(HoudiniNode::ObjNode(ObjNode { id, session }))
    }
}

#[derive(Debug, Clone)]
pub struct SopNode {
    id: ffi::HAPI_NodeId,
    session: Session,
}
#[derive(Debug, Clone)]
pub struct ObjNode {
    id: ffi::HAPI_NodeId,
    session: Session,
}

impl SopNode {}

impl ObjNode {}
