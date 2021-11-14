use std::{ffi::CString, fmt::Formatter};

use log::{debug, warn};

use crate::{
    errors::Result,
    ffi,
    ffi::{AssetInfo, GeoInfo, NodeInfo, ObjectInfo, ParmInfo},
    geometry::Geometry,
    parameter::*,
    session::{CookResult, Session},
};
pub use crate::{
    ffi::raw::{ErrorCode, NodeFlags, NodeType, ParmType, State, StatusType, StatusVerbosity},
    ffi::CookOptions,
};

pub const fn node_type_name(tp: NodeType) -> &'static str {
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

impl ffi::NodeInfo {
    pub fn new(session: &Session, node: NodeHandle) -> Result<Self> {
        let info = crate::ffi::get_node_info(node, &session)?;
        Ok(ffi::NodeInfo {
            inner: info,
            session: session.clone(),
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NodeHandle(pub ffi::raw::HAPI_NodeId, pub(crate) ());

impl NodeHandle {
    pub fn info(&self, session: &Session) -> Result<NodeInfo> {
        NodeInfo::new(session, *self)
    }

    pub fn is_valid(&self, session: &Session) -> Result<bool> {
        let info = self.info(session)?;
        crate::ffi::is_node_valid(&info)
    }

    pub fn to_node(&self, session: &Session) -> Result<HoudiniNode> {
        HoudiniNode::new(session.clone(), *self, None)
    }
}

#[derive(Clone)]
pub struct HoudiniNode {
    pub handle: NodeHandle,
    pub session: Session,
    pub info: NodeInfo, // TODO: Rc Maybe?
}

impl std::fmt::Debug for HoudiniNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HoudiniNode")
            .field("id", &self.handle.0)
            .field("path", &self.path(None).unwrap())
            .finish()
    }
}

impl<'session> HoudiniNode {
    pub(crate) fn new(
        session: Session,
        handle: NodeHandle,
        info: Option<NodeInfo>,
    ) -> Result<Self> {
        let info = match info {
            None => NodeInfo::new(&session, handle)?,
            Some(i) => i,
        };
        Ok(HoudiniNode {
            handle,
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
        let opts;
        let opt = match options {
            None => {
                opts = CookOptions::default();
                &opts
            }
            Some(o) => o,
        };
        crate::ffi::cook_node(self, opt)
    }

    /// In sync mode (single threaded), the error will be available in Err(..) while
    /// in threaded mode the error will be in Ok(..)
    pub fn cook_blocking(&self, options: Option<&CookOptions>) -> Result<CookResult> {
        self.cook(options)?;
        self.session.cook()
    }

    pub fn cook_count(
        &self,
        node_types: NodeType,
        node_flags: NodeFlags,
        recurse: bool,
    ) -> Result<i32> {
        crate::ffi::get_total_cook_count(self, node_types, node_flags, recurse)
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
        let name = CString::new(name)?;
        let label = label.map(|s| CString::new(s).unwrap());
        let id = crate::ffi::create_node(&name, label.as_deref(), &session, parent, cook)?;
        HoudiniNode::new(session, NodeHandle(id, ()), None)
    }

    pub fn create_blocking(
        name: &str,
        label: Option<&str>,
        parent: Option<NodeHandle>,
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        let node = HoudiniNode::create(name, label, parent, session.clone(), cook);
        if node.is_ok() && session.threaded.get() {
            session.cook()?;
        }
        node
    }

    pub fn get_manager_node(session: Session, node_type: NodeType) -> Result<HoudiniNode> {
        let id = crate::ffi::get_manager_node(&session, node_type)?;
        HoudiniNode::new(session, NodeHandle(id, ()), None)
    }

    pub fn get_objects_info(&self) -> Result<Vec<ObjectInfo>> {
        let parent = match self.info.node_type() {
            NodeType::Obj => self.info.parent_id().0,
            _ => self.handle.0,
        };
        let infos = crate::ffi::get_composed_object_list(&self.session, parent)?;
        Ok(infos
            .into_iter()
            .map(|inner| ObjectInfo {
                inner,
                session: &self.session,
            })
            .collect())
    }

    pub fn get_children(
        &self,
        types: NodeType,
        flags: NodeFlags,
        recursive: bool,
    ) -> Result<Vec<NodeHandle>> {
        let ids = crate::ffi::get_compose_child_node_list(self, types, flags, recursive)?;
        Ok(ids.iter().map(|i| NodeHandle(*i, ())).collect())
    }

    /// Search child node by name, relatively expensive, due to retrieving node names
    pub fn node(&self, name: &str) -> Result<Option<HoudiniNode>> {
        let types = self.info.node_type();
        let flags = NodeFlags::Any;
        let nodes = crate::ffi::get_compose_child_node_list(self, types, flags, false)?;
        let handle = nodes.iter().find(|id| {
            let h = NodeHandle(**id, ());
            match h.info(&self.session) {
                Ok(info) => info.name().expect("oops") == name,
                Err(_) => {
                    warn!("Failed to get NodeInfo");
                    false
                }
            }
        });

        match handle {
            None => Ok(None),
            Some(id) => Ok(Some(NodeHandle(*id, ()).to_node(&self.session)?)),
        }
    }

    pub fn parent_node(&self) -> Option<NodeHandle> {
        let h = self.info.parent_id();
        match h.0 > -1 {
            true => Some(h),
            false => None,
        }
    }

    pub fn parameter(&'session self, name: &str) -> Result<Parameter> {
        let parm_info = crate::ffi::ParmInfo::from_parm_name(name, self)?;
        Ok(Parameter::new(self.handle, parm_info))
    }

    pub fn parameters(&'session self) -> Result<Vec<Parameter>> {
        let infos = crate::ffi::get_parameters(self)?;
        Ok(infos
            .into_iter()
            .map(|i| {
                ParmInfo {
                    inner: i,
                    session: self.session.clone(),
                }
                .into_node_parm(self.handle)
            })
            .collect())
    }

    pub fn asset_info(&'session self) -> Result<AssetInfo<'session>> {
        AssetInfo::new(self)
    }
    pub fn check_for_specific_error(&self, error_bits: ErrorCode) -> Result<ErrorCode> {
        crate::ffi::check_for_specific_errors(self, error_bits)
    }

    pub fn cook_result(&self, verbosity: StatusVerbosity) -> Result<String> {
        unsafe { ffi::get_composed_cook_result(self, verbosity) }
    }
    pub fn reset_simulation(&self) -> Result<()> {
        crate::ffi::reset_simulation(self)
    }

    pub fn input_node(&'session self, idx: i32) -> Result<Option<HoudiniNode>> {
        crate::ffi::query_node_input(self, idx).map(|idx| {
            if idx == -1 {
                None
            } else {
                HoudiniNode::new(self.session.clone(), NodeHandle(idx, ()), None).ok()
            }
        })
    }
    pub fn geometry(&'session self) -> Result<Option<Geometry<'session>>> {
        use std::borrow::Cow;
        match self.info.node_type() {
            NodeType::Sop => {
                let info = crate::ffi::get_geo_info(self).map(|inner| GeoInfo {
                    inner,
                    session: &self.session,
                })?;
                Ok(Some(Geometry {
                    node: Cow::Borrowed(self),
                    info,
                }))
            }
            NodeType::Obj => {
                let info = crate::ffi::get_geo_display_info(self).map(|inner| GeoInfo {
                    inner,
                    session: &self.session,
                })?;
                let node = Cow::Owned(info.node_id().to_node(&self.session)?);
                Ok(Some(Geometry { node, info }))
            }
            NodeType(_) => Ok(None),
        }
    }

    // TODO: Make functions generic over HoudiniNode/NodeHandle
    pub fn connect_input(
        &self,
        input_num: i32,
        source: NodeHandle,
        output_num: i32,
    ) -> Result<()> {
        crate::ffi::connect_node_input(
            &self.session, self.handle, input_num, source, output_num)
    }
}
