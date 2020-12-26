use crate::{
    errors::*,
    parameter::*,
    session::{CookResult, Session},
    stringhandle,
    ffi,
    ffi::{ParmInfo, NodeInfo, AssetInfo},
};
pub use crate::{
    ffi::raw::{NodeFlags, NodeType, State, StatusType, StatusVerbosity, ParmType},
    ffi::CookOptions,
};

use std::{
    ffi::CString,
    mem::MaybeUninit,
    ptr::null,
    fmt::Formatter,
    rc::Rc,
};
use log::{debug, warn, log_enabled, Level::Debug};


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

impl std::fmt::Debug for ffi::NodeInfo {
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

impl ffi::NodeInfo {
    pub fn new(session: Session, node: &NodeHandle) -> Result<Self> {
        let info = crate::ffi::get_node_info(node, &session)?;
        Ok(ffi::NodeInfo { inner: info, session })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NodeHandle(pub ffi::raw::HAPI_NodeId);

impl NodeHandle {
    pub fn info(&self, session: Session) -> Result<NodeInfo> {
        NodeInfo::new(session, self)
    }

    pub fn is_valid(&self, session: &Session) -> Result<bool> {
        let info = self.info(session.clone())?;
        crate::ffi::is_node_valid(&info)
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
        crate::ffi::delete_node(self)
    }

    pub fn is_valid(&self) -> Result<bool> {
        Ok(self.info.is_valid())
    }

    pub fn path(&self, relative_to: Option<&HoudiniNode>) -> Result<String> {
        crate::ffi::get_node_path(self, relative_to)
    }

    pub fn cook(&self, options: Option<&CookOptions>) -> Result<()> {
        debug!("Cooking node: {}", self.path(None)?);
        let opt = options.map(|o| o.ptr()).unwrap_or(null());
        crate::ffi::cook_node(self, opt)
    }

    pub fn cook_blocking(&self, options: Option<&CookOptions>) -> Result<CookResult> {
        self.cook(options)?;
        self.session.cook()
    }

    pub fn cook_count(&self, node_types: NodeType, node_flags: NodeFlags) -> Result<i32> {
        let mut count = MaybeUninit::uninit();
        unsafe {
            ffi::raw::HAPI_GetTotalCookCount(
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
            ffi::raw::HAPI_CreateNode(
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

    pub fn get_manager_node(session: Session, node_type: NodeType) -> Result<HoudiniNode> {
        let id = unsafe {
            let mut id = MaybeUninit::uninit();
            ffi::raw::HAPI_GetManagerNodeId(session.ptr(), node_type, id.as_mut_ptr())
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
            ffi::raw::HAPI_ComposeObjectList(
                self.session.ptr(),
                self.handle.0,
                null(),
                count.as_mut_ptr(),
            )
                .result_with_session(|| self.session.clone())?;
            let count = count.assume_init();
            let mut obj_infos = vec![ffi::raw::HAPI_ObjectInfo_Create(); count as usize];
            ffi::raw::HAPI_GetComposedObjectList(
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
            ffi::raw::HAPI_ComposeChildNodeList(
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
            ffi::raw::HAPI_GetComposedChildNodeList(
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
        let parm_info = crate::ffi::ParmInfo::from_parm_name(name, self)?;
        Ok(Parameter::new(self, parm_info))
    }

    pub fn parameters(&self) -> Result<Vec<Parameter<'_>>> {
        let infos = unsafe {
            let mut parms = vec![ffi::raw::HAPI_ParmInfo_Create(); self.info.parm_count() as usize];
            ffi::raw::HAPI_GetParameters(
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
            .map(|i| Parameter::new(self, ParmInfo {
                inner: i,
                session: &self.session,
                name: None,
            }))
            .collect())
    }

    pub fn asset_info(&self) -> Result<AssetInfo<'_>> {
        AssetInfo::new(self)
    }
}
