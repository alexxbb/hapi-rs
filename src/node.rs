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
use std::str::FromStr;
use std::sync::Arc;
use std::{ffi::CString, fmt::Formatter};

use log::{debug, warn};

use crate::pdg::TopNode;
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

/// Types of Houdini manager nodes (contexts).
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum ManagerType {
    Obj,
    Chop,
    Cop,
    Rop,
    Top,
}

impl FromStr for ManagerType {
    type Err = crate::HapiError;

    fn from_str(val: &str) -> Result<Self> {
        match val {
            "Cop2" => Ok(Self::Cop),
            "Chop" => Ok(Self::Chop),
            "Top" => Ok(Self::Top),
            "Object" => Ok(Self::Obj),
            "Driver" => Ok(Self::Rop),
            v => Err(crate::HapiError::internal(format!(
                "Unknown NetworkType: {v}"
            ))),
        }
    }
}

impl From<ManagerType> for NodeType {
    fn from(value: ManagerType) -> Self {
        use ManagerType::*;
        match value {
            Obj => NodeType::Obj,
            Chop => NodeType::Chop,
            Cop => NodeType::Cop,
            Rop => NodeType::Rop,
            Top => NodeType::Top,
        }
    }
}

// Helper function to return all child nodes of specified type
fn get_child_node_list(
    session: &Session,
    parent: impl Into<NodeHandle>,
    types: NodeType,
    flags: NodeFlags,
    recursive: bool,
) -> Result<Vec<NodeHandle>> {
    debug_assert!(session.is_valid());
    let ids =
        crate::ffi::get_compose_child_node_list(session, parent.into(), types, flags, recursive)?;
    Ok(ids.iter().map(|i| NodeHandle(*i)).collect())
}

// Helper function to return all network type nodes.
fn find_networks_nodes(
    session: &Session,
    types: NodeType,
    parent: impl Into<NodeHandle>,
    recursive: bool,
) -> Result<Vec<HoudiniNode>> {
    get_child_node_list(session, parent, types, NodeFlags::Network, recursive).map(|vec| {
        vec.into_iter()
            .map(|handle| handle.to_node(session))
            .collect::<Result<Vec<_>>>()
    })?
}

#[derive(Debug, Clone)]
/// Represents a manager node (OBJ, SOP, etc)
pub struct ManagerNode {
    pub session: Session,
    pub handle: NodeHandle,
    pub node_type: ManagerType,
}

impl ManagerNode {
    /// Find network nodes of given type.
    pub fn find_network_nodes(&self, types: NodeType) -> Result<Vec<HoudiniNode>> {
        find_networks_nodes(&self.session, types, self.handle, true)
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// A lightweight handle to a node. Can not be created manually, use [`HoudiniNode`] instead.
/// Some APIs return a list of such handles for efficiency, for example [`HoudiniNode::find_children_by_type`].
/// Once you found the node you're looking for, upgrade it to a "full" node type.
pub struct NodeHandle(pub(crate) crate::ffi::raw::HAPI_NodeId);

impl From<NodeHandle> for crate::ffi::raw::HAPI_NodeId {
    fn from(h: NodeHandle) -> Self {
        h.0
    }
}

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

    /// Upgrade the handle to HoudiniNode, which has more capabilities.
    pub fn to_node(&self, session: &Session) -> Result<HoudiniNode> {
        HoudiniNode::new(session.clone(), *self, None)
    }

    /// Upgrade the handle to Geometry node.
    pub fn as_geometry_node(&self, session: &Session) -> Result<Option<Geometry>> {
        let info = NodeInfo::new(session, *self)?;
        match info.node_type() {
            NodeType::Sop => Ok(Some(Geometry {
                node: HoudiniNode::new(session.clone(), *self, Some(info))?,
                info: GeoInfo::from_handle(*self, session)?,
            })),
            _ => Ok(None),
        }
    }

    /// If this is a handle to a TOP node, returns a [`TopNode`] type.
    pub fn as_top_node(&self, session: &Session) -> Result<Option<TopNode>> {
        let node = self.to_node(session)?;
        match node.info.node_type() {
            NodeType::Top => Ok(Some(TopNode { node })),
            _ => Ok(None),
        }
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
            .field("type", &self.info.node_type())
            .field("path", &self.path().unwrap())
            .finish()
    }
}

impl From<HoudiniNode> for NodeHandle {
    fn from(n: HoudiniNode) -> Self {
        n.handle
    }
}

impl From<HoudiniNode> for Option<NodeHandle> {
    fn from(n: HoudiniNode) -> Self {
        Some(n.handle)
    }
}

impl From<&HoudiniNode> for Option<NodeHandle> {
    fn from(n: &HoudiniNode) -> Self {
        Some(n.handle)
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

    /// Convert this node instance into [`TopNode`]
    pub fn to_top_node(self) -> Option<TopNode> {
        match self.info.node_type() {
            NodeType::Top => Some(TopNode { node: self }),
            _ => None,
        }
    }
    /// Delete the node in this session.
    pub fn delete(self) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::delete_node(self)
    }

    /// Checks if the node valid (not deleted).
    pub fn is_valid(&self) -> Result<bool> {
        self.handle.is_valid(&self.session)
    }

    pub fn name(&self) -> Result<String> {
        self.info.name()
    }

    /// Returns node's internal path.
    pub fn path(&self) -> Result<String> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_node_path(&self.session, self.handle, None)
    }

    /// Returns node's path relative to another node.
    pub fn path_relative(&self, to: impl Into<Option<NodeHandle>>) -> Result<String> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_node_path(&self.session, self.handle, to.into())
    }

    /// Start cooking the node. This is a non-blocking call if the session is async.
    pub fn cook(&self, options: Option<&CookOptions>) -> Result<()> {
        debug_assert!(self.is_valid()?);
        debug!("Cooking node: {}", self.path()?);
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

    /// Start cooking the node and wait until completed.
    /// In sync mode (single threaded), the error will be available in Err(..) while
    /// in threaded cooking mode the status will be in [`CookResult`]
    pub fn cook_blocking(&self, options: Option<&CookOptions>) -> Result<CookResult> {
        debug_assert!(self.is_valid()?);
        self.cook(options)?;
        self.session.cook()
    }

    /// How many times the node has been cooked.
    pub fn cook_count(
        &self,
        node_types: NodeType,
        node_flags: NodeFlags,
        recurse: bool,
    ) -> Result<i32> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_total_cook_count(self, node_types, node_flags, recurse)
    }

    /// Create a node in the session.
    pub fn create<N: Into<Option<NodeHandle>>>(
        name: &str,
        label: Option<&str>,
        parent: N,
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        debug!("Creating node instance: {}", name);
        debug_assert!(session.is_valid());
        let parent = parent.into();
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
        let id = crate::ffi::create_node(&name, label.as_deref(), &session, parent, cook)?;
        HoudiniNode::new(session, NodeHandle(id), None)
    }

    /// If the node is of Object type, get the information object about it.
    pub fn get_object_info(&self) -> Result<ObjectInfo<'_>> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_object_info(&self.session, self.handle).map(|info| ObjectInfo {
            inner: info,
            session: &self.session,
        })
    }

    /// Returns information objects about this node children.
    pub fn get_objects_info(&self) -> Result<Vec<ObjectInfo>> {
        debug_assert!(self.is_valid()?);
        let parent = match self.info.node_type() {
            NodeType::Obj => NodeHandle(self.info.parent_id().0),
            _ => NodeHandle(self.handle.0),
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
    pub fn find_children_by_type(
        &self,
        types: NodeType,
        flags: NodeFlags,
        recursive: bool,
    ) -> Result<Vec<NodeHandle>> {
        get_child_node_list(&self.session, self, types, flags, recursive)
    }

    /// Get a child node by path.
    pub fn find_child_by_path(&self, relative_path: &str) -> Result<HoudiniNode> {
        self.session
            .find_node_from_path(relative_path, Some(self.handle))
    }

    /// *Search* for child node by name.
    pub fn find_child_by_name(
        &self,
        name: impl AsRef<str>,
        node_type: NodeType,
        recursive: bool,
    ) -> Result<Option<HoudiniNode>> {
        debug_assert!(self.is_valid()?);
        let nodes = self.find_children_by_type(node_type, NodeFlags::Any, recursive)?;
        // TODO: Shortcut if "recursive" is false, search directly by path
        let handle = nodes
            .iter()
            .find(|handle| match handle.info(&self.session) {
                Ok(info) => info.name().expect("oops") == name.as_ref(),
                Err(_) => {
                    warn!("Failed to get NodeInfo");
                    false
                }
            });
        handle
            .map(|handle| handle.to_node(&self.session))
            .transpose()
    }

    /// Return the node's parent.
    pub fn parent_node(&self) -> Option<NodeHandle> {
        let handle = self.info.parent_id();
        (handle.0 > -1).then_some(handle)
    }

    /// Find a parameter on the node by name. Err() means parameter not found.
    pub fn parameter(&self, name: &str) -> Result<Parameter> {
        debug_assert!(self.is_valid()?);
        let parm_info = ParmInfo::from_parm_name(name, self)?;
        Ok(Parameter::new(self.handle, parm_info))
    }

    /// Return all node parameters.
    pub fn parameters(&self) -> Result<Vec<Parameter>> {
        debug_assert!(self.is_valid()?);
        let infos = crate::ffi::get_parameters(self)?;
        Ok(infos
            .into_iter()
            .map(|i| {
                Parameter::new(
                    self.handle,
                    ParmInfo {
                        inner: i,
                        session: self.session.clone(),
                        name: None,
                    },
                )
            })
            .collect())
    }

    /// If node is an HDA, return [`AssetInfo'] about it.
    pub fn asset_info(&'session self) -> Result<AssetInfo<'session>> {
        debug_assert!(self.is_valid()?);
        AssetInfo::new(self)
    }
    /// Recursively check all nodes for a specific error.
    pub fn check_for_specific_error(&self, error_bits: i32) -> Result<ErrorCode> {
        debug_assert!(self.is_valid()?);
        crate::ffi::check_for_specific_errors(self, error_bits)
    }

    /// Compose the cook result string (errors and warnings).
    pub fn cook_result(&self, verbosity: StatusVerbosity) -> Result<String> {
        debug_assert!(self.is_valid()?);
        unsafe { crate::ffi::get_composed_cook_result(self, verbosity) }
    }
    /// Resets the simulation cache of the asset.
    pub fn reset_simulation(&self) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::reset_simulation(self)
    }

    /// Return a node connected to given input.
    pub fn input_node(&self, idx: i32) -> Result<Option<HoudiniNode>> {
        debug_assert!(self.is_valid()?);
        crate::ffi::query_node_input(self, idx).map(|idx| {
            if idx == -1 {
                None
            } else {
                HoudiniNode::new(self.session.clone(), NodeHandle(idx), None).ok()
            }
        })
    }

    /// Give the node a new name.
    pub fn rename(&self, new_name: impl AsRef<str>) -> Result<()> {
        let name = CString::new(new_name.as_ref())?;
        crate::ffi::rename_node(self, &name)
    }

    /// Saves the node and all its contents to file
    pub fn save_to_file(&self, file: impl AsRef<Path>) -> Result<()> {
        debug_assert!(self.is_valid()?);
        let filename = CString::new(file.as_ref().to_string_lossy().to_string())?;
        crate::ffi::save_node_to_file(self.handle, &self.session, &filename)
    }

    /// Loads and creates a previously saved node and all its contents from given file.
    pub fn load_from_file(
        session: &Session,
        parent: impl Into<Option<NodeHandle>>,
        label: &str,
        cook: bool,
        file: impl AsRef<Path>,
    ) -> Result<HoudiniNode> {
        debug_assert!(session.is_valid());
        let filename = CString::new(file.as_ref().to_string_lossy().to_string())?;
        let label = CString::new(label)?;
        let id = crate::ffi::load_node_from_file(parent.into(), session, &label, &filename, cook)?;
        NodeHandle(id).to_node(session)
    }

    /// Returns a node preset as bytes.
    pub fn get_preset(&self, name: &str, preset_type: PresetType) -> Result<Vec<i8>> {
        debug_assert!(self.is_valid()?);
        let name = CString::new(name)?;
        crate::ffi::get_preset(&self.session, self.handle, &name, preset_type)
    }

    /// Set the preset data to the node.
    pub fn set_preset(&self, name: &str, preset_type: PresetType, data: &[i8]) -> Result<()> {
        debug_assert!(self.is_valid()?);
        let name = CString::new(name)?;
        crate::ffi::set_preset(&self.session, self.handle, &name, preset_type, data)
    }

    /// Return Geometry for this node if it's a SOP node,
    /// otherwise find a child SOP node with display flag and return.
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

    /// Search this node for TOP networks
    pub fn find_top_networks(&self) -> Result<Vec<HoudiniNode>> {
        find_networks_nodes(&self.session, NodeType::Top, self, true)
    }

    /// How many geometry output nodes there is inside an Object or SOP node.
    pub fn number_of_geo_outputs(&self) -> Result<i32> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_output_geo_count(self)
    }

    /// Return all output nodes as Geometry.
    pub fn geometry_outputs(&self) -> Result<Vec<Geometry>> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_output_geos(self).map(|vec| {
            vec.into_iter()
                .map(|inner| {
                    NodeHandle(inner.nodeId)
                        .to_node(&self.session)
                        .map(|node| Geometry {
                            node,
                            info: GeoInfo { inner },
                        })
                })
                .collect::<Result<Vec<_>>>()
        })?
    }

    /// If node is an Object, return it's transform.
    pub fn get_transform(
        &self,
        rst_order: Option<RSTOrder>,
        relative_to: impl Into<Option<NodeHandle>>,
    ) -> Result<Transform> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_object_transform(
            &self.session,
            self.handle,
            relative_to.into(),
            rst_order.unwrap_or(RSTOrder::Default),
        )
        .map(|inner| Transform { inner })
    }

    /// Set transform on the Object
    pub fn set_transform(&self, transform: &TransformEuler) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::set_object_transform(&self.session, self.handle, &transform.inner)
    }

    /// Set keyframes animation on the Object.
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

    /// Connect output of another node into an input on this node.
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

    /// Get the nodes currently connected to the given node at the output index.
    pub fn output_connected_nodes(
        &self,
        output_index: i32,
        search_subnets: bool,
    ) -> Result<Vec<NodeHandle>> {
        debug_assert!(self.is_valid()?);
        crate::ffi::query_node_output_connected_nodes(self, output_index, search_subnets)
    }

    /// Disconnect a given input index.
    pub fn disconnect_input(&self, input_index: i32) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::disconnect_node_input(self, input_index)
    }

    /// Disconnect a given output index.
    pub fn disconnect_outputs(&self, output_index: i32) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::disconnect_node_outputs(self, output_index)
    }

    /// Set display flag on this node.
    pub fn set_display_flag(&self, on: bool) -> Result<()> {
        debug_assert!(self.is_valid()?);
        crate::ffi::set_node_display(&self.session, self.handle, on)
    }

    /// Get the name of a node's input.
    pub fn get_input_name(&self, input_index: i32) -> Result<String> {
        crate::ffi::get_node_input_name(self, input_index)
    }
}

#[cfg(test)]
mod tests {
    use crate::session::tests::with_session;
    use std::iter::repeat_with;

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
            let n = node.find_child_by_path("geo/point_attr").unwrap();
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
        dbg!(&geo.node);
        let child = geo
            .node
            .parent_node()
            .unwrap()
            .to_node(&session)
            .unwrap()
            .find_child_by_name("add_color", NodeType::Sop, false)
            .unwrap();
        assert!(child.is_some());
        let child = node.find_child_by_path("geo/add_color");
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
            assert_eq!(p.get(1).unwrap(), 5.0);
        }
    }

    #[test]
    fn get_set_preset() {
        with_session(|session| {
            let node = session
                .create_node("Object/null", "get_set_parent", None)
                .unwrap();
            if let Parameter::Float(p) = node.parameter("scale").unwrap() {
                assert_eq!(p.get(0).unwrap(), 1.0);
                let save = node.get_preset("test", PresetType::Binary).unwrap();
                p.set(0, 2.0).unwrap();
                node.set_preset("test", PresetType::Binary, &save).unwrap();
                assert_eq!(p.get(0).unwrap(), 1.0);
            }
        });
    }

    #[test]
    fn concurrent_parm_access() {
        use crate::session::*;

        fn set_parm_value(parm: &Parameter) {
            match parm {
                Parameter::Float(parm) => {
                    let val: [f32; 3] = std::array::from_fn(|_| fastrand::f32());
                    parm.set_array(val).unwrap()
                }
                Parameter::Int(parm) => {
                    let values: Vec<_> = repeat_with(|| fastrand::i32(0..10))
                        .take(parm.wrap.info.size() as usize)
                        .collect();
                    parm.set_array(&values).unwrap()
                }
                Parameter::String(parm) => {
                    let values: Vec<String> = (0..parm.wrap.info.size())
                        .into_iter()
                        .map(|_| repeat_with(fastrand::alphanumeric).take(10).collect())
                        .collect();
                    parm.set_array(values).unwrap()
                }
                Parameter::Button(parm) => parm.press_button().unwrap(),
                Parameter::Other(_) => {}
            };
        }

        fn get_parm_value(parm: &Parameter) {
            match parm {
                Parameter::Float(parm) => {
                    parm.get(0).unwrap();
                }
                Parameter::Int(parm) => {
                    parm.get(0).unwrap();
                }
                Parameter::String(parm) => {
                    parm.get(0).unwrap();
                }
                Parameter::Button(_) => {}
                Parameter::Other(_) => {}
            };
        }

        let session = quick_session(Some(
            &SessionOptionsBuilder::default().threaded(true).build(),
        ))
        .unwrap();
        let lib = session
            .load_asset_file("otls/hapi_parms.hda")
            .expect("loaded asset");
        let node = lib.try_create_first().expect("hapi_parm node");
        node.cook_blocking(None).unwrap();
        let parameters = node.parameters().expect("parameters");
        std::thread::scope(|scope| {
            for _ in 0..3 {
                scope.spawn(|| {
                    for _ in 0..parameters.len() {
                        let i = fastrand::usize(..parameters.len());
                        let parm = &parameters[i];
                        if fastrand::bool() {
                            set_parm_value(parm);
                            node.cook(None).unwrap();
                        } else {
                            get_parm_value(parm);
                            node.cook(None).unwrap();
                        }
                    }
                });
            }
        });
    }
}
