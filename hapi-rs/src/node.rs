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
    ffi::raw::{
        ErrorCode, NodeFlags, NodeType, ParmType, RSTOrder, State, StatusType, StatusVerbosity,
    },
    ffi::{CookOptions, Transform, TransformEuler},
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
        let info = crate::ffi::get_node_info(node, session)?;
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

impl From<HoudiniNode> for NodeHandle {
    fn from(n: HoudiniNode) -> Self {
        n.handle
    }
}

impl From<&HoudiniNode> for NodeHandle {
    fn from(n: &HoudiniNode) -> Self {
        n.handle
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

    pub fn path(&self, relative_to: Option<NodeHandle>) -> Result<String> {
        crate::ffi::get_node_path(&self.session, self.handle, relative_to)
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

    pub fn create<H: Into<NodeHandle>>(
        name: &str,
        label: Option<&str>,
        parent: Option<H>,
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        debug!("Creating node: {}", name);
        if parent.is_none() && !name.contains('/') {
            warn!("Node name must be fully qualified if parent is not specified");
        } else if parent.is_some() && name.contains("/") {
            warn!("Cannot use fully qualified node name with parent");
        }
        let name = CString::new(name)?;
        let label = label.map(|s| CString::new(s).unwrap());
        let id = crate::ffi::create_node(
            &name,
            label.as_deref(),
            &session,
            parent.map(|t| t.into()),
            cook,
        )?;
        HoudiniNode::new(session, NodeHandle(id, ()), None)
    }

    pub fn create_blocking<H: Into<NodeHandle>>(
        name: &str,
        label: Option<&str>,
        parent: Option<H>,
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        let node = HoudiniNode::create(name, label, parent, session.clone(), cook);
        if node.is_ok() && session.threaded {
            session.cook()?;
        }
        node
    }

    pub fn get_manager_node(session: Session, node_type: NodeType) -> Result<HoudiniNode> {
        let id = crate::ffi::get_manager_node(&session, node_type)?;
        HoudiniNode::new(session, NodeHandle(id, ()), None)
    }

    pub fn get_object_info(&self) -> Result<ObjectInfo<'_>> {
        crate::ffi::get_object_info(&self.session, self.handle).map(|info| ObjectInfo {
            inner: info,
            session: &self.session,
        })
    }

    pub fn get_objects_info(&self) -> Result<Vec<ObjectInfo>> {
        let parent = match self.info.node_type() {
            NodeType::Obj => NodeHandle(self.info.parent_id().0, ()),
            _ => NodeHandle(self.handle.0, ()),
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

    pub fn parameter(&self, name: &str) -> Result<Parameter> {
        let parm_info = crate::ffi::ParmInfo::from_parm_name(name, self)?;
        Ok(Parameter::new(self.handle, parm_info))
    }

    pub fn parameters(&self) -> Result<Vec<Parameter>> {
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

    pub fn input_node(&self, idx: i32) -> Result<Option<HoudiniNode>> {
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

    pub fn get_transform(
        &self,
        rst_order: Option<RSTOrder>,
        relative_to: Option<NodeHandle>,
    ) -> Result<Transform> {
        crate::ffi::get_object_transform(
            &self.session,
            self.handle,
            relative_to,
            rst_order.unwrap_or(RSTOrder::Default),
        )
            .map(|inner| Transform { inner })
    }

    pub fn set_transform(&self, transform: &TransformEuler) -> Result<()> {
        crate::ffi::set_object_transform(&self.session, self.handle, &transform.inner)
    }

    pub fn connect_input<H: Into<NodeHandle>>(
        &self,
        input_num: i32,
        source: H,
        output_num: i32,
    ) -> Result<()> {
        crate::ffi::connect_node_input(
            &self.session,
            self.handle,
            input_num,
            source.into(),
            output_num,
        )
    }

    pub fn set_display_flag(&self, on: bool) -> Result<()> {
        crate::ffi::set_node_display(&self.session, self.handle, on)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::tests::with_session;

    #[test]
    fn node_flags() {
        with_session(|session| {
            let sop = session.create_node("Object/geo", None, None).unwrap();
            let sphere = session
                .create_node("sphere", None, Some(sop.handle))
                .unwrap();
            let _box = session.create_node("box", None, Some(sop.handle)).unwrap();
            _box.set_display_flag(true).unwrap();
            assert!(!sphere.geometry().unwrap().unwrap().info.is_display_geo());
            sphere.set_display_flag(true).unwrap();
            assert!(!_box.geometry().unwrap().unwrap().info.is_display_geo());
        });
    }

    #[test]
    fn node_transform() {
        with_session(|session| {
            let obj = session
                .create_node_blocking("Object/null", None, None)
                .unwrap();
            let t = obj.get_transform(None, None).unwrap();
            assert_eq!(t.position(), [0.0, 0.0, 0.0]);
            assert_eq!(t.scale(), [1.0, 1.0, 1.0]);
            assert_eq!(t.rst_order(), RSTOrder::Default);
            obj.set_transform(
                &TransformEuler::default()
                    .with_position([0.0, 1.0, 0.0])
                    .with_rotation([45.0, 0.0, 0.0]),
            )
               .unwrap();
            obj.cook(None).unwrap();
            assert!(obj.get_object_info().unwrap().has_transform_changed());
            let t = obj.get_transform(None, None).unwrap();
            assert_eq!(t.position(), [0.0, 1.0, 0.0]);
        });
    }
}
