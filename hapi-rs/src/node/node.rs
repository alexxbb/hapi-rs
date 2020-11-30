use super::info::*;
use crate::{
    auto::bindings as ffi,
    auto::rusty::{
        NodeFlags, NodeFlagsBits, NodeType, NodeTypeBits, State, StatusType, StatusVerbosity,
    },
    cookoptions::CookOptions,
    errors::*,
    session::{CookResult, Session},
    stringhandle,
};
use std::{
    ffi::CString,
    mem::MaybeUninit,
    pin::Pin,
    ptr::null,
    sync::Arc,
    task::{Context, Poll},
};

use log::{debug, log_enabled, Level::Debug};

#[derive(Debug, Clone)]
pub struct HoudiniNode {
    pub(crate) id: ffi::HAPI_NodeId,
    pub session: Session,
}

impl HoudiniNode {
    pub fn delete(self) -> Result<()> {
        unsafe {
            ffi::HAPI_DeleteNode(self.session.ptr(), self.id)
                .result_with_session(|| self.session.clone())
        }
    }

    pub fn info(&self) -> Result<NodeInfo<'_>> {
        unsafe {
            let info = ffi::HAPI_NodeInfo_Create();
            ffi::HAPI_GetNodeInfo(self.session.ptr(), self.id, &info as *const _ as *mut _)
                .result_with_session(|| self.session.clone())?;
            Ok(NodeInfo::from_ffi(info, self))
        }
    }

    pub fn is_valid(&self) -> Result<bool> {
        Ok(self.info()?.is_valid)
        // unsafe {
        //     ffi::HAPI_IsNodeValid()
        // }
    }

    pub fn path(&self, relative_to: Option<HoudiniNode>) -> Result<String> {
        unsafe {
            let mut sh = MaybeUninit::uninit();
            ffi::HAPI_GetNodePath(
                self.session.ptr(),
                self.id,
                relative_to.map(|n| n.id).unwrap_or(-1),
                sh.as_mut_ptr(),
            )
            .result_with_session(|| self.session.clone())?;
            stringhandle::get_string(sh.assume_init(), &self.session)
        }
    }

    /// https://github.com/sideeffects/HoudiniEngineForUnity/blob/5b2d34bd5a04513288f4991048bf9c5ecceacac5/Plugins/HoudiniEngineUnity/Scripts/Asset/HEU_HoudiniAsset.cs#L1536
    pub fn cook(&self, options: Option<CookOptions>) -> Result<()> {
        if log_enabled!(Debug) {
            debug!("Cooking node: {}", self.path(None)?)
        }
        let opt = options.map(|o| o.ptr()).unwrap_or(null());
        unsafe {
            ffi::HAPI_CookNode(self.session.ptr(), self.id, opt)
                .result_with_session(|| self.session.clone())?;
        }
        Ok(())
    }

    pub fn cook_blocking(&self, options: Option<CookOptions>) -> Result<CookResult> {
        self.cook(options)?;
        self.session.cook_result()
    }

    pub fn cook_count(&self, node_types: NodeFlagsBits, node_flags: NodeFlagsBits) -> Result<i32> {
        let mut count = MaybeUninit::uninit();
        unsafe {
            ffi::HAPI_GetTotalCookCount(
                self.session.ptr(),
                self.id,
                node_types,
                node_flags,
                true as i8,
                count.as_mut_ptr(),
            )
            .result_with_session(|| self.session.clone())?;
            Ok(count.assume_init())
        }
    }

    pub fn create(
        name: &str,
        label: Option<&str>,
        parent: Option<HoudiniNode>,
        session: &Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        let mut id = MaybeUninit::uninit();
        let parent = parent.map_or(-1, |n| n.id);
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
            let n = HoudiniNode {
                id: id.assume_init(),
                session: session.clone(),
            };
            Ok(n)
        }
    }

    pub fn create_blocking(
        name: &str,
        label: Option<&str>,
        parent: Option<HoudiniNode>,
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        let node = HoudiniNode::create(name, label, parent, &session, cook);
        if node.is_ok() && session.unsync {
            loop {
                match session.get_status(StatusType::CookState)? {
                    State::Ready => break,
                    _ => {}
                }
            }
        }
        node
    }
}
