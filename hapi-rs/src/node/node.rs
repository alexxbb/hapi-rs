use super::info::*;
use crate::{
    auto::bindings as ffi,
    errors::*,
    parameter::*,
    session::{CookOptions, CookResult, Session},
    asset::AssetInfo,
    stringhandle,
};

pub use super::info::NodeInfo;
pub use crate::ffi::{NodeFlags, NodeType};
use ffi::{State, StatusType, StatusVerbosity};
use std::{
    ffi::CString,
    mem::MaybeUninit,
    pin::Pin,
    ptr::null,
    sync::Arc,
    task::{Context, Poll},
};

use crate::auto::bindings::ParmType;
use log::{debug, warn, log_enabled, Level::Debug};
use std::fmt::Formatter;
use std::rc::Rc;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NodeHandle(pub ffi::HAPI_NodeId);

impl NodeHandle {
    pub fn info(&self, session: &Session) -> Result<NodeInfo> {
        NodeInfo::new(session.clone(), self)
    }

    pub fn is_valid(&self, session: &Session) -> Result<bool> {
        let uid = self.info(session)?.unique_node_id();
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
    pub handle: NodeHandle,
    pub session: Session,
    pub info: NodeInfo,
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
    pub(crate) fn new(session: Session, hdl: NodeHandle, info: Option<NodeInfo>) -> Result<Self> {
        let mut info = match info {
            None => NodeInfo::new(session.clone(), &hdl)?,
            Some(i) => i,
        };
        Ok(HoudiniNode {
            handle: hdl,
            session,
            info,
        })
    }
    pub fn delete(self) -> Result<()> {
        unsafe {
            ffi::HAPI_DeleteNode(self.session.ptr(), self.handle.0)
                .result_with_session(|| self.session.clone())
        }
    }

    pub fn is_valid(&self) -> Result<bool> {
        Ok(self.info.is_valid())
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

    pub fn cook_count(&self, node_types: NodeType, node_flags: NodeFlags) -> Result<i32> {
        let mut count = MaybeUninit::uninit();
        unsafe {
            ffi::HAPI_GetTotalCookCount(
                self.session.ptr(),
                self.handle.0,
                node_types.0,
                node_flags.0,
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
        parent: Option<NodeHandle>,
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        debug!("Creating node: {}", name);
        if parent.is_none() && !name.contains('/') {
            warn!("Node name must be fully qualified if parent is not specified");
        }
        let mut id = MaybeUninit::uninit();
        let parent = parent.map_or(-1, |n| n.0);
        let mut label_ptr: *const std::os::raw::c_char = null();
        let id = unsafe {
            let mut tmp;
            if let Some(lb) = label {
                tmp = CString::from_vec_unchecked(lb.into());
                label_ptr = tmp.as_ptr();
            }
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
        HoudiniNode::new(session, NodeHandle(id), None)
    }

    pub fn create_blocking(
        name: &str,
        label: Option<&str>,
        parent: Option<NodeHandle>,
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

    pub fn get_manager_node(session: Session, node_type: ffi::NodeType) -> Result<HoudiniNode> {
        let id = unsafe {
            let mut id = MaybeUninit::uninit();
            ffi::HAPI_GetManagerNodeId(session.ptr(), node_type, id.as_mut_ptr())
                .result_with_session(|| session.clone())?;
            id.assume_init()
        };
        HoudiniNode::new(session, NodeHandle(id), None)
    }

    pub fn get_object_nodes(&self) -> Result<Vec<NodeHandle>> {
        let node_id = match self.info.node_type() {
            NodeType::Obj => self.info.parent_id(),
            _ => self.handle.clone(),
        };
        let obj_infos = unsafe {
            let mut count = MaybeUninit::uninit();
            ffi::HAPI_ComposeObjectList(
                self.session.ptr(),
                self.handle.0,
                null(),
                count.as_mut_ptr(),
            )
            .result_with_session(|| self.session.clone())?;
            let count = count.assume_init();
            let mut obj_infos = vec![ffi::HAPI_ObjectInfo_Create(); count as usize];
            ffi::HAPI_GetComposedObjectList(
                self.session.ptr(),
                self.handle.0,
                obj_infos.as_mut_ptr(),
                0,
                count,
            )
            .result_with_session(|| self.session.clone())?;
            obj_infos
        };

        Ok(obj_infos.iter().map(|i| NodeHandle(i.nodeId)).collect())
    }

    pub fn get_children(
        &self,
        types: NodeType,
        flags: NodeFlags,
        recursive: bool,
    ) -> Result<Vec<NodeHandle>> {
        let ids = unsafe {
            let mut count = MaybeUninit::uninit();
            ffi::HAPI_ComposeChildNodeList(
                self.session.ptr(),
                self.handle.0,
                types.0,
                flags.0,
                recursive as i8,
                count.as_mut_ptr(),
            )
            .result_with_session(|| self.session.clone())?;

            let count = count.assume_init();
            let mut obj_infos = vec![0i32; count as usize];
            ffi::HAPI_GetComposedChildNodeList(
                self.session.ptr(),
                self.handle.0,
                obj_infos.as_mut_ptr(),
                count,
            )
            .result_with_session(|| self.session.clone())?;
            obj_infos
        };

        Ok(ids.iter().map(|i| NodeHandle(*i)).collect())
    }

    pub fn parent_node(&self) -> Result<NodeHandle> {
        Ok(self.info.parent_id())
    }

    pub fn parameter(&self, name: &str) -> Result<Parameter<'_>> {
        let name = CString::new(name)?;
        let parm_info = crate::parameter::ParmInfo::from_parm_name(name, self)?;
        Ok(Parameter::new(self, parm_info))
    }

    pub fn parameters(&self) -> Result<Vec<Parameter<'_>>> {
        let infos = unsafe {
            let mut parms = vec![ffi::HAPI_ParmInfo_Create(); self.info.parm_count() as usize];
            ffi::HAPI_GetParameters(
                self.session.ptr(),
                self.handle.0,
                parms.as_mut_ptr(),
                0,
                self.info.parm_count(),
            )
            .result_with_session(|| self.session.clone())?;
            parms
        };

        Ok(infos
            .into_iter()
            .map(|i| Parameter::new(self, ParmInfo{
                inner: i,
                session: &self.session,
                name: None
            }))
            .collect())
    }

    pub fn asset_info(&self) -> Result<AssetInfo<'_>> {
        AssetInfo::new(self)
    }
}
