//! Manipulating Houdini nodes and networks, getting geometry and parameters
//!
//! Any Houdini nodes is represented as [`HoudiniNode`] struct and all node-related functions are exposed as
//! methods on that struct. It has a public `info` filed with [`NodeInfo`] with details about the node.
//!
//! Nodes can be created with [`Session::create_node`]
//!
//! HoudiniNode is ['Clone'], [`Sync`] and [`Send`]
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::{ffi::CString, ffi::OsStr, fmt::Formatter};

use log::debug;

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
                "Unknown ManagerType::{v}"
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
    let ids =
        crate::ffi::get_compose_child_node_list(session, parent.into(), types, flags, recursive)?;
    Ok(ids.into_iter().map(NodeHandle).collect())
}

// Helper function to return all network type nodes.
fn find_networks_nodes(
    session: &Session,
    types: NodeType,
    parent: impl Into<NodeHandle>,
    recursive: bool,
) -> Result<Vec<HoudiniNode>> {
    debug_assert!(session.is_valid());
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

    /// Return children nodes of this network.
    pub fn get_children(&self) -> Result<Vec<NodeHandle>> {
        get_child_node_list(
            &self.session,
            self.handle,
            NodeType::from(self.node_type),
            NodeFlags::Any,
            false,
        )
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
/// A lightweight handle to a node. Can not be created manually, use [`HoudiniNode`] instead.
/// Some APIs return a list of such handles for efficiency, for example [`HoudiniNode::find_children_by_type`].
/// Once you found the node you're looking for, upgrade it to a "full" node type.
pub struct NodeHandle(pub(crate) crate::ffi::raw::HAPI_NodeId);

impl From<NodeHandle> for crate::ffi::raw::HAPI_NodeId {
    fn from(h: NodeHandle) -> Self {
        h.0
    }
}

impl AsRef<NodeHandle> for HoudiniNode {
    fn as_ref(&self) -> &NodeHandle {
        &self.handle
    }
}

impl AsRef<NodeHandle> for NodeHandle {
    fn as_ref(&self) -> &NodeHandle {
        self
    }
}

impl Default for NodeHandle {
    fn default() -> Self {
        NodeHandle(-1)
    }
}

impl NodeHandle {
    /// Retrieve info about the node this handle belongs to
    pub fn info(&self, session: &Session) -> Result<NodeInfo> {
        NodeInfo::new(session, *self)
    }

    /// Returns node's internal path.
    pub fn path(&self, session: &Session) -> Result<String> {
        debug_assert!(self.is_valid(session)?, "Invalid {:?}", self);
        crate::ffi::get_node_path(session, *self, None)
    }

    /// Returns node's path relative to another node.
    pub fn path_relative(
        &self,
        session: &Session,
        to: impl Into<Option<NodeHandle>>,
    ) -> Result<String> {
        debug_assert!(self.is_valid(session)?, "Invalid {:?}", self);
        crate::ffi::get_node_path(session, *self, to.into())
    }

    /// Check if the handle is valid (node wasn't deleted)
    pub fn is_valid(&self, session: &Session) -> Result<bool> {
        let info = self.info(session)?;
        crate::ffi::is_node_valid(session, &info.0)
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
            .field(
                "path",
                &self.path().expect("[HoudiniNode::Debug] node path"),
            )
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

impl HoudiniNode {
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
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::delete_node(self.handle, &self.session)
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
        self.handle.path(&self.session)
    }

    /// Returns node's path relative to another node.
    pub fn path_relative(&self, to: impl Into<Option<NodeHandle>>) -> Result<String> {
        self.handle.path_relative(&self.session, to)
    }

    /// Start cooking the node. This is a non-blocking call if the session is async.
    pub fn cook(&self) -> Result<()> {
        debug!("Start cooking node: {}", self.path()?);
        debug_assert!(self.is_valid()?);
        crate::ffi::cook_node(self, &CookOptions::default())
    }

    /// Start cooking the node and wait until completed.
    /// In sync mode (single threaded), the error will be available in Err(..) while
    /// in threaded cooking mode the status will be in [`CookResult`]
    pub fn cook_blocking(&self) -> Result<CookResult> {
        debug!("Start cooking node: {}", self.path()?);
        debug_assert!(self.is_valid()?);
        crate::ffi::cook_node(self, &CookOptions::default())?;
        self.session.cook()
    }

    /// Start cooking with options and wait for result if blocking = true.
    pub fn cook_with_options(&self, options: &CookOptions, blocking: bool) -> Result<CookResult> {
        debug!("Start cooking node: {}", self.path()?);
        debug_assert!(self.is_valid()?);
        crate::ffi::cook_node(self, options)?;
        if blocking {
            self.session.cook()
        } else {
            Ok(CookResult::Succeeded)
        }
    }

    /// How many times this node has been cooked.
    pub fn cook_count(
        &self,
        node_types: NodeType,
        node_flags: NodeFlags,
        recurse: bool,
    ) -> Result<i32> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_total_cook_count(self, node_types, node_flags, recurse)
    }

    /// If the node is of Object type, get the information object about it.
    pub fn get_object_info(&self) -> Result<ObjectInfo<'_>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::get_object_info(&self.session, self.handle)
            .map(|info| ObjectInfo(info, (&self.session).into()))
    }

    /// Get a new NodeInfo even for this node.
    pub fn get_info(&self) -> Result<NodeInfo> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        self.handle.info(&self.session)
    }

    /// Returns information objects about this node children.
    pub fn get_objects_info(&self) -> Result<Vec<ObjectInfo>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        let parent = match self.info.node_type() {
            NodeType::Obj => self.info.parent_id(),
            _ => self.handle,
        };
        let infos = crate::ffi::get_composed_object_list(&self.session, parent)?;
        Ok(infos
            .into_iter()
            .map(|inner| ObjectInfo(inner, (&self.session).into()))
            .collect())
    }

    /// Find all children of this node by type.
    pub fn find_children_by_type(
        &self,
        types: NodeType,
        flags: NodeFlags,
        recursive: bool,
    ) -> Result<Vec<NodeHandle>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        get_child_node_list(&self.session, self, types, flags, recursive)
    }

    /// Get all children of the node, not recursively.
    pub fn get_children(&self) -> Result<Vec<NodeHandle>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        get_child_node_list(&self.session, self, NodeType::Any, NodeFlags::Any, false)
    }

    /// Get a child node by path.
    pub fn get_child_by_path(&self, relative_path: &str) -> Result<Option<HoudiniNode>> {
        self.session
            .get_node_from_path(relative_path, Some(self.handle))
    }

    /// Get the node ids for the objects being instanced by an Instance OBJ node.
    pub fn get_instanced_object_ids(&self) -> Result<Vec<NodeHandle>> {
        crate::ffi::get_instanced_object_ids(self)
    }

    /// *Search* for child node by name.
    pub fn find_child_node(
        &self,
        name: impl AsRef<str>,
        recursive: bool,
    ) -> Result<Option<HoudiniNode>> {
        debug_assert!(self.is_valid()?);
        if !recursive {
            return self.get_child_by_path(name.as_ref());
        }
        for handle in self.find_children_by_type(NodeType::Any, NodeFlags::Any, recursive)? {
            let info = handle.info(&self.session)?;
            if info.name()? == name.as_ref() {
                return Ok(Some(HoudiniNode::new(
                    self.session.clone(),
                    handle,
                    Some(info),
                )?));
            }
        }
        Ok(None)
    }

    /// Given if Self is an asset or a subnet SOP node, get its output node at index.
    pub fn get_sop_output_node(&self, index: i32) -> Result<NodeHandle> {
        debug_assert!(self.is_valid()?);
        crate::ffi::get_sop_output_node(&self.session, self.handle, index)
    }

    /// Return the node's parent.
    pub fn parent_node(&self) -> Option<NodeHandle> {
        let handle = self.info.parent_id();
        (handle.0 > -1).then_some(handle)
    }

    /// Find a parameter on the node by name. Err() means parameter not found.
    pub fn parameter(&self, name: &str) -> Result<Parameter> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        let parm_info = ParmInfo::from_parm_name(name, self)?;
        Ok(Parameter::new(self.handle, parm_info))
    }

    /// Find a parameter with a specific tag
    pub fn parameter_with_tag(&self, tag: &str) -> Result<Option<Parameter>> {
        let tag = CString::new(tag)?;
        match crate::ffi::get_parm_with_tag(self, &tag)? {
            -1 => Ok(None),
            h => {
                let parm_info =
                    ParmInfo::from_parm_handle(ParmHandle(h), self.handle, &self.session)?;
                Ok(Some(Parameter::new(self.handle, parm_info)))
            }
        }
    }

    /// Return all node parameters.
    pub fn parameters(&self) -> Result<Vec<Parameter>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        let infos = crate::ffi::get_parameters(self)?;
        Ok(infos
            .into_iter()
            .map(|info| {
                Parameter::new(self.handle, ParmInfo::new(info, self.session.clone(), None))
            })
            .collect())
    }

    /// If node is an HDA, return [`AssetInfo'] about it.
    pub fn asset_info(&self) -> Result<AssetInfo> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        Ok(AssetInfo(
            crate::ffi::get_asset_info(self)?,
            self.session.clone().into(),
        ))
    }
    /// Recursively check all nodes for a specific error.
    pub fn check_for_specific_error(&self, error_bits: i32) -> Result<ErrorCode> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::check_for_specific_errors(self, error_bits)
    }

    /// Compose the cook result (errors and warnings) of all nodes in the network into a string.
    pub fn get_composed_cook_result_string(&self, verbosity: StatusVerbosity) -> Result<String> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        unsafe { crate::ffi::get_composed_cook_result(self, verbosity) }
    }

    /// Get the cook errors and warnings on this node as a string
    pub fn get_cook_result_string(&self, verbosity: StatusVerbosity) -> Result<String> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        let bytes = crate::ffi::get_node_cook_result(self, verbosity)?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }
    /// Resets the simulation cache of the asset.
    pub fn reset_simulation(&self) -> Result<()> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::reset_simulation(self)
    }

    /// Return a node connected to given input.
    pub fn input_node(&self, idx: i32) -> Result<Option<HoudiniNode>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
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
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        let name = CString::new(new_name.as_ref())?;
        crate::ffi::rename_node(self, &name)
    }

    /// Saves the node and all its contents to file
    pub fn save_to_file(&self, file: impl AsRef<Path>) -> Result<()> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        let filename = CString::new(file.as_ref().to_string_lossy().to_string())?;
        crate::ffi::save_node_to_file(self.handle, &self.session, &filename)
    }

    /// Loads and creates a previously saved node and all its contents from given file.
    pub fn load_from_file(
        session: &Session,
        parent: impl Into<Option<NodeHandle>>,
        label: &str,
        cook: bool,
        file: impl AsRef<OsStr>,
    ) -> Result<HoudiniNode> {
        debug_assert!(session.is_valid());
        debug!("Loading node from file {:?}", file.as_ref());
        let filename = CString::new(file.as_ref().to_string_lossy().to_string())?;
        let label = CString::new(label)?;
        let id = crate::ffi::load_node_from_file(parent.into(), session, &label, &filename, cook)?;
        NodeHandle(id).to_node(session)
    }

    /// Returns a node preset as bytes.
    pub fn get_preset(&self, name: &str, preset_type: PresetType) -> Result<Vec<i8>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        let name = CString::new(name)?;
        crate::ffi::get_preset(&self.session, self.handle, &name, preset_type)
    }

    /// Set the preset data to the node.
    pub fn set_preset(&self, name: &str, preset_type: PresetType, data: &[i8]) -> Result<()> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        let name = CString::new(name)?;
        crate::ffi::set_preset(&self.session, self.handle, &name, preset_type, data)
    }

    /// Return Geometry for this node if it's a SOP node,
    /// otherwise find a child SOP node with display flag and return.
    pub fn geometry(&self) -> Result<Option<Geometry>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        match self.info.node_type() {
            NodeType::Sop => Ok(Some(Geometry {
                node: self.clone(),
                info: GeoInfo::from_node(self)?,
            })),
            NodeType::Obj => {
                let info = crate::ffi::get_geo_display_info(self).map(GeoInfo)?;
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
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::get_output_geo_count(self)
    }

    /// Get names of each HDA output
    pub fn get_output_names(&self) -> Result<Vec<String>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::get_output_names(self)
    }

    /// Return all output nodes as Geometry.
    pub fn geometry_output_nodes(&self) -> Result<Vec<Geometry>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::get_output_geos(self).map(|vec| {
            vec.into_iter()
                .map(|inner| {
                    NodeHandle(inner.nodeId)
                        .to_node(&self.session)
                        .map(|node| Geometry {
                            node,
                            info: GeoInfo(inner),
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
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::get_object_transform(
            &self.session,
            self.handle,
            relative_to.into(),
            rst_order.unwrap_or(RSTOrder::Default),
        )
        .map(Transform)
    }

    /// Set transform on the Object
    pub fn set_transform(&self, transform: &TransformEuler) -> Result<()> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::set_object_transform(&self.session, self.handle, &transform.0)
    }

    /// Set keyframes animation on the Object.
    pub fn set_transform_anim_curve(
        &self,
        component: TransformComponent,
        keys: &[KeyFrame],
    ) -> Result<()> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
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
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
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
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::query_node_output_connected_nodes(self, output_index, search_subnets)
    }

    /// Disconnect a given input index.
    pub fn disconnect_input(&self, input_index: i32) -> Result<()> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::disconnect_node_input(self, input_index)
    }

    /// Disconnect a given output index.
    pub fn disconnect_outputs(&self, output_index: i32) -> Result<()> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::disconnect_node_outputs(self, output_index)
    }

    /// Set display flag on this node.
    pub fn set_display_flag(&self, on: bool) -> Result<()> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::set_node_display(&self.session, self.handle, on)
    }

    /// Get the name of a node's input.
    pub fn get_input_name(&self, input_index: i32) -> Result<String> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::get_node_input_name(self, input_index)
    }

    /// Get the ids of the message nodes specified in the HDA Type Properties
    pub fn get_message_nodes(&self) -> Result<Vec<NodeHandle>> {
        debug_assert!(self.is_valid()?, "Invalid node: {}", self.path()?);
        crate::ffi::get_message_node_ids(self)
    }
}
