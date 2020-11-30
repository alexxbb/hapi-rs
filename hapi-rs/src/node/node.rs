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
use std::cell::RefCell;
use std::fmt::Formatter;

#[derive(Debug, Clone)]
pub struct NodeHandle(pub(crate) ffi::HAPI_NodeId);

pub(crate) fn read_node_info(
    session: &Session,
    handle: &NodeHandle,
    info: &mut NodeInfo,
) -> Result<()> {
    unsafe {
        ffi::HAPI_GetNodeInfo(session.ptr(), handle.0, &mut info.inner as *mut _)
            .result_with_session(|| session.clone())?;
    }
    Ok(())
}
impl NodeHandle {
    pub fn info(&self, session: &Session) -> Result<NodeInfo> {
        let mut info = NodeInfo::default();
        read_node_info(session, self, &mut info)?;
        Ok(info)
    }

    pub fn fill_info(&self, session: &Session, info: &mut NodeInfo) -> Result<()> {
        read_node_info(session, &self, info)
    }

    pub fn is_valid(&self, session: &Session) -> Result<bool> {
        let uid = self.info(session)?.unique_houdini_node_id();
        unsafe {
            let mut answer = MaybeUninit::uninit();
            ffi::HAPI_IsNodeValid(session.ptr(), self.0, uid, answer.as_mut_ptr())
                .result_with_session(|| session.clone())?;
            Ok(answer.assume_init() == 1)
        }
    }
}

#[derive(Clone)]
pub struct HoudiniNode {
    pub(crate) handle: NodeHandle,
    pub session: Session,
}

impl std::fmt::Debug for HoudiniNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HoudiniNode")
            .field("id", &self.handle.0)
            .field("path", &self.path(None).unwrap())
            .finish()
    }
}

impl HoudiniNode {
    pub(crate) fn new(session: Session, hdl: NodeHandle) -> Result<Self> {
        Ok(HoudiniNode {
            handle: hdl,
            session,
        })
    }
    pub fn delete(self) -> Result<()> {
        unsafe {
            ffi::HAPI_DeleteNode(self.session.ptr(), self.handle.0)
                .result_with_session(|| self.session.clone())
        }
    }

    pub fn info(&self) -> Result<NodeInfo> {
        self.handle.info(&self.session)
    }

    pub fn is_valid(&self) -> Result<bool> {
        Ok(self.info()?.is_valid())
    }

    pub fn path(&self, relative_to: Option<HoudiniNode>) -> Result<String> {
        unsafe {
            let mut sh = MaybeUninit::uninit();
            ffi::HAPI_GetNodePath(
                self.session.ptr(),
                self.handle.0,
                relative_to.map(|n| n.handle.0).unwrap_or(-1),
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
            ffi::HAPI_CookNode(self.session.ptr(), self.handle.0, opt)
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
                self.handle.0,
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
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        let mut id = MaybeUninit::uninit();
        let parent = parent.map_or(-1, |n| n.handle.0);
        let mut label_ptr: *const std::os::raw::c_char = null();
        let id = unsafe {
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
            id.assume_init()
        };
        HoudiniNode::new(session, NodeHandle(id))
    }

    pub fn create_blocking(
        name: &str,
        label: Option<&str>,
        parent: Option<HoudiniNode>,
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        let node = HoudiniNode::create(name, label, parent, session.clone(), cook);
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

    pub fn get_manager_node(session: Session, node_type: i32) -> Result<HoudiniNode> {
        let id = unsafe {
            let mut id = MaybeUninit::uninit();
            ffi::HAPI_GetManagerNodeId(session.ptr(), node_type, id.as_mut_ptr())
                .result_with_session(|| session.clone())?;
            id.assume_init()
        };
        HoudiniNode::new(session, NodeHandle(id))
    }

    pub fn parent_node(&self) -> Option<HoudiniNode> {
        todo!()
    }
}
