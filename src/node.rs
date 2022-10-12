//! Manipulating Houdini nodes and networks, getting geometry and parameters
//!
//! Any Houdini nodes is represented as [`HoudiniNode`] struct and all node-related functions are exposed as
//! methods on that struct. It has a public `info` filed with [`NodeInfo`] with details about the node.
//!
//! Nodes can be created directly with [`HoudiniNode::create()`] functions but a recommended way is
//! through the session object: [`Session::create_node`]
//!
//! HoudiniNode is [`Sync`] and [`Send`]
use std::path::Path;
use std::sync::Arc;
use std::{ffi::CString, fmt::Formatter};

use log::{debug, warn};

pub use crate::{
    errors::Result,
    ffi::{AssetInfo, GeoInfo, KeyFrame, NodeInfo, ObjectInfo, ParmInfo},
    geometry::Geometry,
    parameter::*,
    session::{CookResult, Session},
};
pub use crate::{
    ffi::raw::{
        ErrorCode, NodeFlags, NodeType, PresetType, RSTOrder, State, StatusType, StatusVerbosity,
        TransformComponent,
    },
    ffi::{CookOptions, Transform, TransformEuler},
};

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match *self {
            NodeType::Sop => "Sop",
            NodeType::Obj => "Obj",
            NodeType::Rop => "Rop",
            NodeType::Dop => "Dop",
            NodeType::Cop => "Cop",
            NodeType::Shop => "Shop",
            NodeType::Vop => "Vop",
            NodeType::Chop => "Chop",
            _ => "Unknown",
        })
    }
}

impl NodeInfo {
    pub fn new(session: &Session, node: NodeHandle) -> Result<Self> {
        let info = crate::ffi::get_node_info(node, session)?;
        Ok(NodeInfo { inner: info })
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// A handle to a node. Can not be created manually, use [`HoudiniNode`] instead.
pub struct NodeHandle(pub(crate) crate::ffi::raw::HAPI_NodeId, pub(crate) ());

impl NodeHandle {
    /// Retrieve info about the node this handle belongs to
    pub fn info(&self, session: &Session) -> Result<NodeInfo> {
        NodeInfo::new(session, *self)
    }

    /// Check if the handle is valid (node wasn't deleted)
    pub fn is_valid(&self, session: &Session) -> Result<bool> {
        let info = self.info(session)?;
        crate::ffi::is_node_valid(session, &info.inner)
    }

    /// Upgrade the handle to HoudiniNode, which has more capabilities
    pub fn to_node(&self, session: &Session) -> Result<HoudiniNode> {
        HoudiniNode::new(session.clone(), *self, None)
    }
}

#[derive(Clone)]
/// Represents a Houdini node
pub struct HoudiniNode {
    pub handle: NodeHandle,
    pub session: Session,
    pub info: Arc<NodeInfo>,
}

impl PartialEq for HoudiniNode {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle && self.session == other.session
    }
}

impl std::fmt::Debug for HoudiniNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HoudiniNode")
            .field("id", &self.handle.0)
            .field("path", &self.path().unwrap())
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
        let info = Arc::new(match info {
            None => NodeInfo::new(&session, handle)?,
            Some(i) => i,
        });
        Ok(HoudiniNode {
            handle,
            session,
            info,
        })
    }
    pub fn delete(self) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::delete_node(self)
    }

    pub fn is_valid(&self) -> Result<bool> {
        self.handle.is_valid(&self.session)
    }

    pub fn name(&self) -> Result<String> {
        self.info.name(&self.session)
    }

    pub fn path(&self) -> Result<String> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_node_path(&self.session, self.handle, None)
    }

    pub fn path_relative(&self, to: Option<NodeHandle>) -> Result<String> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_node_path(&self.session, self.handle, to)
    }

    pub fn cook(&self, options: Option<&CookOptions>) -> Result<()> {
        debug!("Cooking node: {}", self.path()?);
        debug_assert!(self.is_valid()?);
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
    /// in threaded cooking mode the status will be in [`CookResult`]
    pub fn cook_blocking(&self, options: Option<&CookOptions>) -> Result<CookResult> {
        debug_assert!(self.is_valid()?);
        self.cook(options)?;
        self.session.cook()
    }

    pub fn cook_count(
        &self,
        node_types: NodeType,
        node_flags: NodeFlags,
        recurse: bool,
    ) -> Result<i32> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_total_cook_count(self, node_types, node_flags, recurse)
    }

    pub fn create<H: Into<NodeHandle>>(
        name: &str,
        label: Option<&str>,
        parent: Option<H>,
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        debug_assert!(session.is_valid());
        // assert!(!parent.is_none() && !name.contains('/'));
        debug_assert!(
            parent.is_some() || name.contains('/'),
            "Node name must be fully qualified if parent is not specified"
        );
        debug_assert!(
            !(parent.is_some() && name.contains('/')),
            "Cannot use fully qualified node name with parent"
        );
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

    pub fn get_manager_node(session: &Session, node_type: NodeType) -> Result<HoudiniNode> {
        debug_assert!(session.is_valid());
        let id = crate::ffi::get_manager_node(session, node_type)?;
        HoudiniNode::new(session.clone(), NodeHandle(id, ()), None)
    }

    pub fn get_object_info(&self) -> Result<ObjectInfo<'_>> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_object_info(&self.session, self.handle).map(|info| ObjectInfo {
            inner: info,
            session: &self.session,
        })
    }

    pub fn get_objects_info(&self) -> Result<Vec<ObjectInfo>> {
        debug_assert!(self.is_valid()?);
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

    /// Find all children of this node by type.
    pub fn get_children(
        &self,
        types: NodeType,
        flags: NodeFlags,
        recursive: bool,
    ) -> Result<Vec<NodeHandle>> {
        debug_assert!(self.is_valid()?);
        let ids = crate::ffi::get_compose_child_node_list(
            &self.session,
            self.handle,
            types,
            flags,
            recursive,
        )?;
        Ok(ids.iter().map(|i| NodeHandle(*i, ())).collect())
    }

    /// Get a child node by path.
    pub fn get_child(&self, relative_path: &str) -> Result<HoudiniNode> {
        self.session.find_node(relative_path, Some(self.handle))
    }

    /// *Search* for child node by name.
    pub fn find_child(
        &self,
        name: impl AsRef<str>,
        node_type: NodeType,
    ) -> Result<Option<HoudiniNode>> {
        debug_assert!(self.is_valid()?);
        match self.parent_node() {
            None => Ok(None),
            Some(parent) => {
                let flags = NodeFlags::Any;
                let nodes = crate::ffi::get_compose_child_node_list(
                    &self.session,
                    parent,
                    node_type,
                    flags,
                    false,
                )?;
                let handle = nodes.iter().find(|id| {
                    let h = NodeHandle(**id, ());
                    match h.info(&self.session) {
                        Ok(info) => info.name(&self.session).expect("oops") == name.as_ref(),
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
        debug_assert!(self.is_valid()?);
        let parm_info = ParmInfo::from_parm_name(name, self)?;
        Ok(Parameter::new(self.handle, parm_info))
    }

    pub fn parameters(&self) -> Result<Vec<Parameter>> {
        debug_assert!(self.is_valid()?);
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
        debug_assert!(self.is_valid()?);
        AssetInfo::new(self)
    }
    pub fn check_for_specific_error(&self, error_bits: ErrorCode) -> Result<ErrorCode> {
        debug_assert!(self.is_valid()?);
        crate::ffi::check_for_specific_errors(self, error_bits)
    }

    pub fn cook_result(&self, verbosity: StatusVerbosity) -> Result<String> {
        debug_assert!(self.is_valid()?);
        unsafe { crate::ffi::get_composed_cook_result(self, verbosity) }
    }
    pub fn reset_simulation(&self) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::reset_simulation(self)
    }

    pub fn input_node(&self, idx: i32) -> Result<Option<HoudiniNode>> {
        debug_assert!(self.is_valid()?);
        crate::ffi::query_node_input(self, idx).map(|idx| {
            if idx == -1 {
                None
            } else {
                HoudiniNode::new(self.session.clone(), NodeHandle(idx, ()), None).ok()
            }
        })
    }

    pub fn rename(&self, new_name: impl AsRef<str>) -> Result<()> {
        let name = CString::new(new_name.as_ref())?;
        crate::ffi::rename_node(self, &name)
    }

    pub fn save_to_file(&self, file: impl AsRef<Path>) -> Result<()> {
        debug_assert!(self.is_valid()?);
        let filename = CString::new(file.as_ref().to_string_lossy().to_string())?;
        crate::ffi::save_node_to_file(self.handle, &self.session, &filename)
    }

    pub fn load_from_file(
        session: &Session,
        parent: Option<NodeHandle>,
        label: &str,
        cook: bool,
        file: impl AsRef<Path>,
    ) -> Result<HoudiniNode> {
        debug_assert!(session.is_valid());
        let filename = CString::new(file.as_ref().to_string_lossy().to_string())?;
        let label = CString::new(label)?;
        let id = crate::ffi::load_node_from_file(parent, session, &label, &filename, cook)?;
        NodeHandle(id, ()).to_node(session)
    }

    pub fn get_preset(&self, name: &str, preset_type: PresetType) -> Result<Vec<i8>> {
        debug_assert!(self.is_valid()?);
        let name = CString::new(name)?;
        crate::ffi::get_preset(&self.session, self.handle, &name, preset_type)
    }

    pub fn set_preset(&self, name: &str, preset_type: PresetType, data: &[i8]) -> Result<()> {
        debug_assert!(self.is_valid()?);
        let name = CString::new(name)?;
        crate::ffi::set_preset(&self.session, self.handle, &name, preset_type, data)
    }

    pub fn geometry(&self) -> Result<Option<Geometry>> {
        debug_assert!(self.is_valid()?);
        match self.info.node_type() {
            NodeType::Sop => Ok(Some(Geometry {
                node: self.clone(),
                info: GeoInfo::from_node(self)?,
            })),
            NodeType::Obj => {
                let info = crate::ffi::get_geo_display_info(self).map(|inner| GeoInfo { inner })?;
                Ok(Some(Geometry {
                    node: info.node_id().to_node(&self.session)?,
                    info,
                }))
            }
            _ => Ok(None),
        }
    }

    #[inline]
    pub fn number_of_geo_outputs(&self) -> Result<i32> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_output_geo_count(self)
    }

    pub fn geometry_outputs(&self) -> Result<Vec<Geometry>> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_output_geos(self).map(|vec| {
            vec.into_iter()
                .map(|inner| {
                    NodeHandle(inner.nodeId, ())
                        .to_node(&self.session)
                        .map(|node| Geometry {
                            node,
                            info: GeoInfo { inner },
                        })
                })
                .collect::<Result<Vec<_>>>()
        })?
    }

    pub fn get_transform(
        &self,
        rst_order: Option<RSTOrder>,
        relative_to: Option<NodeHandle>,
    ) -> Result<Transform> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_object_transform(
            &self.session,
            self.handle,
            relative_to,
            rst_order.unwrap_or(RSTOrder::Default),
        )
        .map(|inner| Transform { inner })
    }

    pub fn set_transform(&self, transform: &TransformEuler) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::set_object_transform(&self.session, self.handle, &transform.inner)
    }

    pub fn set_transform_anim_curve(
        &self,
        component: TransformComponent,
        keys: &[KeyFrame],
    ) -> Result<()> {
        debug_assert!(self.is_valid()?);
        let keys =
            unsafe { std::mem::transmute::<&[KeyFrame], &[crate::ffi::raw::HAPI_Keyframe]>(keys) };
        crate::ffi::set_transform_anim_curve(&self.session, self.handle, component, keys)
    }

    pub fn connect_input<H: Into<NodeHandle>>(
        &self,
        input_num: i32,
        source: H,
        output_num: i32,
    ) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::connect_node_input(
            &self.session,
            self.handle,
            input_num,
            source.into(),
            output_num,
        )
    }

    pub fn output_connected_nodes(
        &self,
        output_index: i32,
        search_subnets: bool,
    ) -> Result<Vec<NodeHandle>> {
        debug_assert!(self.is_valid()?);
        crate::ffi::query_node_output_connected_nodes(self, output_index, search_subnets)
    }

    pub fn disconnect_input(&self, input_index: i32) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::disconnect_node_input(self, input_index)
    }

    pub fn disconnect_outputs(&self, output_index: i32) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::disconnect_node_outputs(self, output_index)
    }

    pub fn set_display_flag(&self, on: bool) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::set_node_display(&self.session, self.handle, on)
    }

    pub fn get_input_name(&self, input_index: i32) -> Result<String> {
        crate::ffi::get_node_input_name(self, input_index)
    }
}

#[cfg(test)]
mod tests {
    use crate::session::tests::with_session;

    use super::*;

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
    fn node_inputs_and_outputs() {
        with_session(|session| {
            let node = session.create_node("Object/hapi_geo", None, None).unwrap();
            let geo = node.geometry().unwrap().unwrap();
            let mut input = geo.node.input_node(0).unwrap();
            while let Some(ref n) = input {
                assert!(n.is_valid().unwrap());
                input = n.input_node(0).unwrap();
            }
            let outputs = geo.node.output_connected_nodes(0, false).unwrap();
            assert!(outputs.is_empty());
            let n = node.get_child("geo/point_attr").unwrap();
            assert_eq!(
                n.get_input_name(0).unwrap(),
                "Geometry to Process with Wrangle"
            );
        })
    }

    #[test]
    fn node_find_siblings() {
        let session = crate::session::quick_session(None).unwrap();
        let node = session.create_node("Object/hapi_geo", None, None).unwrap();
        let geo = node.geometry().unwrap().unwrap();
        let child = geo.node.find_child("add_color", NodeType::Sop).unwrap();
        assert!(child.is_some());
        let child = node.get_child("geo/add_color");
        assert!(child.is_ok());
    }

    #[test]
    fn node_transform() {
        with_session(|session| {
            let obj = session
                .create_node("Object/null", "node_transform", None)
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

    #[test]
    fn save_and_load() {
        with_session(|session| {
            let cam = session.create_node("Object/cam", "ToSave", None).unwrap();
            let tmp = std::env::temp_dir().join("node");
            cam.save_to_file(&tmp).expect("save_to_file");
            let new = HoudiniNode::load_from_file(session, None, "loaded_cam", true, &tmp)
                .expect("load_from_file");
            std::fs::remove_file(&tmp).unwrap();
            cam.delete().unwrap();
            new.delete().unwrap();
        });
    }

    #[test]
    fn number_of_geo_outputs() {
        with_session(|session| {
            let node = session.create_node("Object/hapi_geo", None, None).unwrap();
            assert_eq!(node.number_of_geo_outputs(), Ok(2));
            let infos = node.geometry_outputs().unwrap();
            assert_eq!(infos.len(), 2);
        });
    }

    #[test]
    fn set_transform_anim() {
        let session = crate::session::quick_session(None).unwrap();
        let bone = session.create_node("Object/bone", None, None).unwrap();
        let ty = [
            KeyFrame {
                time: 0.0,
                value: 0.0,
                in_tangent: 0.0,
                out_tangent: 0.0,
            },
            KeyFrame {
                time: 1.0,
                value: 5.0,
                in_tangent: 0.0,
                out_tangent: 0.0,
            },
        ];
        bone.set_transform_anim_curve(TransformComponent::Ty, &ty)
            .unwrap();
        session.set_time(1.0).unwrap();
        if let Parameter::Float(p) = bone.parameter("ty").unwrap() {
            assert_eq!(p.get_value().unwrap(), &[0.0, 5.0, 0.0]);
        }
    }

    #[test]
    fn get_set_preset() {
        with_session(|session| {
            let node = session
                .create_node("Object/null", "get_set_parent", None)
                .unwrap();
            if let Parameter::Float(p) = node.parameter("scale").unwrap() {
                assert_eq!(p.get_value().unwrap(), &[1.0]);
                let save = node.get_preset("test", PresetType::Binary).unwrap();
                p.set_value(&[2.0]).unwrap();
                node.set_preset("test", PresetType::Binary, &save).unwrap();
                assert_eq!(p.get_value().unwrap(), &[1.0]);
            }
        });
    }
}
