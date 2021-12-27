use std::sync::Arc;
use std::{ffi::CString, fmt::Formatter};

use log::{debug, warn};

pub use crate::{
    errors::Result,
    ffi::{AssetInfo, GeoInfo, MaterialInfo, NodeInfo, ObjectInfo, ParmInfo, KeyFrame},
    geometry::Geometry,
    parameter::*,
    session::{CookResult, Session},
};
pub use crate::{
    ffi::raw::{
        ErrorCode, NodeFlags, NodeType, PresetType, RSTOrder, State, StatusType,
        StatusVerbosity, TransformComponent,
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

impl crate::ffi::NodeInfo {
    pub fn new(session: &Session, node: NodeHandle) -> Result<Self> {
        let info = crate::ffi::get_node_info(node, session)?;
        Ok(crate::ffi::NodeInfo {
            inner: info,
            session: session.clone(),
        })
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NodeHandle(pub crate::ffi::raw::HAPI_NodeId, pub(crate) ());

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
    pub info: Arc<NodeInfo>,
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
        } else if parent.is_some() && name.contains('/') {
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

    pub fn get_manager_node(session: &Session, node_type: NodeType) -> Result<HoudiniNode> {
        let id = crate::ffi::get_manager_node(session, node_type)?;
        HoudiniNode::new(session.clone(), NodeHandle(id, ()), None)
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
        unsafe { crate::ffi::get_composed_cook_result(self, verbosity) }
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

    pub fn save_to_file(&self, file: impl AsRef<std::path::Path>) -> Result<()> {
        let filename = CString::new(file.as_ref().to_string_lossy().to_string())?;
        crate::ffi::save_node_to_file(self.handle, &self.session, &filename)
    }

    pub fn load_from_file(
        session: &Session,
        parent: Option<NodeHandle>,
        label: &str,
        cook: bool,
        file: impl AsRef<std::path::Path>,
    ) -> Result<HoudiniNode> {
        let filename = CString::new(file.as_ref().to_string_lossy().to_string())?;
        let label = CString::new(label)?;
        let id = crate::ffi::load_node_from_file(parent, session, &label, &filename, cook)?;
        NodeHandle(id, ()).to_node(session)
    }

    pub fn get_preset(&self, name: &str, preset_type: PresetType) -> Result<Vec<i8>> {
        let name = CString::new(name)?;
        crate::ffi::get_preset(&self.session, self.handle, &name, preset_type)
    }

    pub fn set_preset(&self, name: &str, preset_type: PresetType, data: &[i8]) -> Result<()> {
        let name = CString::new(name)?;
        crate::ffi::set_preset(&self.session, self.handle, &name, preset_type, data)
    }

    pub fn geometry(&self) -> Result<Option<Geometry>> {
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
            NodeType(_) => Ok(None),
        }
    }

    #[inline]
    pub fn number_of_geo_outputs(&self) -> Result<i32> {
        crate::ffi::get_output_geo_count(self)
    }

    pub fn geometry_outputs(&self) -> Result<Vec<Geometry>> {
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

    pub fn set_transform_anim_curve(
        &self,
        component: TransformComponent,
        keys: &[KeyFrame],
    ) -> Result<()> {
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
    fn node_transform() {
        with_session(|session| {
            let obj = session
                .create_node_blocking("Object/null", "node_transform", None)
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
            let otl = crate::session::tests::OTLS.get("geometry").unwrap();
            let lib = session.load_asset_file(otl).unwrap();
            let node = lib.try_create_first().unwrap();
            assert_eq!(node.number_of_geo_outputs(), Ok(2));
            let infos = node.geometry_outputs().unwrap();
            assert_eq!(infos.len(), 2);
        });
    }

    #[test]
    fn set_transform_anim() {
        let session = crate::session::quick_session(None).unwrap();
        let bone = session
            .create_node_blocking("Object/bone", None, None)
            .unwrap();
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
                .create_node_blocking("Object/null", "get_set_parent", None)
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
