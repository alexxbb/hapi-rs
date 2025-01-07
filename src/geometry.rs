//! Access to geometry data, attributes, reading and writing geometry to and from disk
//!
//!

use crate::attribute::*;
use crate::errors::Result;
pub use crate::ffi::{
    enums::*, AttributeInfo, BoxInfo, CookOptions, CurveInfo, GeoInfo, InputCurveInfo, PartInfo,
    SphereInfo, Transform, VolumeInfo, VolumeTileInfo, VolumeVisualInfo,
};
use crate::material::Material;
use crate::node::{HoudiniNode, NodeHandle};
use crate::stringhandle::StringArray;
use crate::volume::{Tile, VolumeBounds, VolumeStorage};
use std::ffi::{CStr, CString};

#[derive(Debug, Clone)]
/// Represents a SOP node with methods for manipulating geometry.
pub struct Geometry {
    pub node: HoudiniNode,
    pub(crate) info: GeoInfo,
}

/// In-memory geometry format
#[derive(Debug)]
pub enum GeoFormat {
    Geo,
    Bgeo,
    Obj,
}

#[derive(Debug)]
/// Face materials
pub enum Materials {
    /// Material was assigned at object level or all faces on geometry share the same material
    Single(Material),
    /// Materials assigned per-face
    Multiple(Vec<Material>),
}

impl GeoFormat {
    const fn as_cstr(&self) -> &'static CStr {
        unsafe {
            CStr::from_bytes_with_nul_unchecked(match *self {
                GeoFormat::Geo => b".geo\0",
                GeoFormat::Bgeo => b".bgeo\0",
                GeoFormat::Obj => b".obj\0",
            })
        }
    }
}

/// Common geometry attribute names
#[derive(Debug)]
pub enum AttributeName {
    Cd,
    P,
    N,
    Uv,
    TangentU,
    TangentV,
    Scale,
    Name,
    User(CString),
}

impl TryFrom<&str> for AttributeName {
    type Error = std::ffi::NulError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        CString::new(value).map(AttributeName::User)
    }
}

impl TryFrom<String> for AttributeName {
    type Error = std::ffi::NulError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        CString::new(value).map(AttributeName::User)
    }
}

impl TryFrom<&CStr> for AttributeName {
    type Error = std::convert::Infallible;

    fn try_from(value: &CStr) -> std::result::Result<Self, Self::Error> {
        Ok(AttributeName::User(value.to_owned()))
    }
}

impl From<AttributeName> for CString {
    fn from(name: AttributeName) -> Self {
        macro_rules! cstr {
            ($attr:expr) => {
                unsafe { CStr::from_bytes_with_nul_unchecked($attr).to_owned() }
            };
        }
        match name {
            AttributeName::Cd => cstr!(crate::raw::HAPI_ATTRIB_COLOR),
            AttributeName::P => cstr!(crate::raw::HAPI_ATTRIB_POSITION),
            AttributeName::N => cstr!(crate::raw::HAPI_ATTRIB_NORMAL),
            AttributeName::Uv => cstr!(crate::raw::HAPI_ATTRIB_UV),
            AttributeName::TangentU => cstr!(crate::raw::HAPI_ATTRIB_TANGENT),
            AttributeName::TangentV => cstr!(crate::raw::HAPI_ATTRIB_TANGENT2),
            AttributeName::Scale => cstr!(crate::raw::HAPI_ATTRIB_SCALE),
            AttributeName::Name => cstr!(crate::raw::HAPI_ATTRIB_NAME),
            AttributeName::User(val) => val,
        }
    }
}

impl Geometry {
    /// Get geometry partition info by index.
    pub fn part_info(&self, part_id: i32) -> Result<PartInfo> {
        self.assert_node_cooked();
        crate::ffi::get_part_info(&self.node, part_id).map(PartInfo)
    }

    pub fn volume_info(&self, part_id: i32) -> Result<VolumeInfo> {
        self.assert_node_cooked();
        crate::ffi::get_volume_info(&self.node, part_id).map(VolumeInfo)
    }

    pub fn set_volume_info(&self, part_id: i32, info: &VolumeInfo) -> Result<()> {
        crate::ffi::set_volume_info(&self.node, part_id, &info.0)
    }

    #[inline(always)]
    fn assert_node_cooked(&self) {
        debug_assert!(
            crate::ffi::get_node_info(self.node.handle, &self.node.session)
                .expect("NodeInfo")
                .totalCookCount
                > 0,
            "Node not cooked"
        );
    }

    pub fn volume_bounds(&self, part_id: i32) -> Result<VolumeBounds> {
        self.assert_node_cooked();
        crate::ffi::get_volume_bounds(&self.node, part_id)
    }
    pub fn get_volume_visual_info(&self, part_id: i32) -> Result<VolumeVisualInfo> {
        crate::ffi::get_volume_visual_info(&self.node, part_id).map(VolumeVisualInfo)
    }

    /// Get information about Node's geometry.
    /// Note: The node must be cooked before calling this method.
    pub fn geo_info(&self) -> Result<GeoInfo> {
        self.assert_node_cooked();
        GeoInfo::from_node(&self.node)
    }

    pub fn set_part_info(&self, info: &PartInfo) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        crate::ffi::set_part_info(&self.node, info)
    }

    pub fn box_info(&self, part_id: i32) -> Result<BoxInfo> {
        self.assert_node_cooked();
        crate::ffi::get_box_info(self.node.handle, &self.node.session, part_id).map(BoxInfo)
    }

    pub fn sphere_info(&self, part_id: i32) -> Result<SphereInfo> {
        self.assert_node_cooked();
        crate::ffi::get_sphere_info(self.node.handle, &self.node.session, part_id).map(SphereInfo)
    }

    pub fn set_curve_info(&self, part_id: i32, info: &CurveInfo) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        crate::ffi::set_curve_info(&self.node, part_id, info)
    }

    pub fn set_input_curve_info(&self, part_id: i32, info: &InputCurveInfo) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        crate::ffi::set_input_curve_info(&self.node, part_id, info)
    }

    pub fn get_input_curve_info(&self, part_id: i32) -> Result<InputCurveInfo> {
        debug_assert!(self.node.is_valid()?);
        crate::ffi::get_input_curve_info(&self.node, part_id).map(InputCurveInfo)
    }

    pub fn set_input_curve_positions(&self, part_id: i32, positions: &[f32]) -> Result<()> {
        crate::ffi::set_input_curve_positions(
            &self.node,
            part_id,
            positions,
            0,
            positions.len() as i32,
        )
    }

    pub fn set_input_curve_transform(
        &self,
        part_id: i32,
        positions: &[f32],
        rotation: &[f32],
        scale: &[f32],
    ) -> Result<()> {
        crate::ffi::set_input_curve_transform(&self.node, part_id, positions, rotation, scale)
    }

    pub fn set_curve_counts(&self, part_id: i32, count: &[i32]) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        crate::ffi::set_curve_counts(&self.node, part_id, count)
    }

    pub fn set_curve_knots(&self, part_id: i32, knots: &[f32]) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        crate::ffi::set_curve_knots(&self.node, part_id, knots)
    }

    pub fn set_vertex_list(&self, part_id: i32, list: impl AsRef<[i32]>) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        crate::ffi::set_geo_vertex_list(&self.node, part_id, list.as_ref())
    }

    pub fn set_face_counts(&self, part_id: i32, list: impl AsRef<[i32]>) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        crate::ffi::set_geo_face_counts(&self.node, part_id, list.as_ref())
    }

    pub fn update(&mut self) -> Result<()> {
        self.info = self.geo_info()?;
        Ok(())
    }

    pub fn curve_info(&self, part_id: i32) -> Result<CurveInfo> {
        self.assert_node_cooked();
        crate::ffi::get_curve_info(&self.node, part_id).map(CurveInfo)
    }

    /// Retrieve the number of vertices for each curve in the part.
    pub fn curve_counts(&self, part_id: i32, start: i32, length: i32) -> Result<Vec<i32>> {
        self.assert_node_cooked();
        crate::ffi::get_curve_counts(&self.node, part_id, start, length)
    }

    /// Retrieve the orders for each curve in the part if the curve has varying order.
    pub fn curve_orders(&self, part_id: i32, start: i32, length: i32) -> Result<Vec<i32>> {
        self.assert_node_cooked();
        crate::ffi::get_curve_orders(&self.node, part_id, start, length)
    }

    /// Retrieve the knots of the curves in this part.
    pub fn curve_knots(&self, part_id: i32, start: i32, length: i32) -> Result<Vec<f32>> {
        self.assert_node_cooked();
        crate::ffi::get_curve_knots(&self.node, part_id, start, length)
    }

    /// Get array containing the vertex-point associations where the
    /// ith element in the array is the point index the ith vertex
    /// associates with.
    pub fn vertex_list(&self, part: &PartInfo) -> Result<Vec<i32>> {
        self.assert_node_cooked();
        crate::ffi::get_geo_vertex_list(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            0,
            part.vertex_count(),
        )
    }

    pub fn partitions(&self) -> Result<Vec<PartInfo>> {
        self.assert_node_cooked();
        (0..self.geo_info()?.part_count())
            .map(|part| self.part_info(part))
            .collect()
    }

    pub fn get_face_counts(&self, part: &PartInfo) -> Result<Vec<i32>> {
        self.assert_node_cooked();
        crate::ffi::get_face_counts(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            0,
            part.face_count(),
        )
    }

    /// Return material nodes applied to geometry.
    pub fn get_materials(&self, part: &PartInfo) -> Result<Option<Materials>> {
        self.assert_node_cooked();
        let (all_the_same, mats) = crate::ffi::get_material_node_ids_on_faces(
            &self.node.session,
            self.node.handle,
            part.face_count(),
            part.part_id(),
        )?;
        if all_the_same {
            if mats[0] == -1 {
                Ok(None)
            } else {
                let mat_node = NodeHandle(mats[0]);
                let info = crate::ffi::get_material_info(&self.node.session, mat_node)?;
                Ok(Some(Materials::Single(Material {
                    session: self.node.session.clone(),
                    info,
                })))
            }
        } else {
            let session = self.node.session.clone();
            let mats = mats
                .into_iter()
                .map(|id| {
                    crate::ffi::get_material_info(&session, NodeHandle(id)).map(|info| Material {
                        session: session.clone(),
                        info,
                    })
                })
                .collect::<Result<Vec<_>>>();
            mats.map(|vec| Some(Materials::Multiple(vec)))
        }
    }

    /// Get geometry group names by type.
    pub fn get_group_names(&self, group_type: GroupType) -> Result<StringArray> {
        self.assert_node_cooked();
        let count = match group_type {
            GroupType::Point => self.info.point_group_count(),
            GroupType::Prim => self.info.primitive_group_count(),
            GroupType::Edge => self.info.edge_group_count(),
            _ => unreachable!("Impossible GroupType value"),
        };
        crate::ffi::get_group_names(&self.node, group_type, count)
    }

    pub fn get_edge_count_of_edge_group(&self, group: &str, part_id: i32) -> Result<i32> {
        self.assert_node_cooked();
        let group = CString::new(group)?;
        crate::ffi::get_edge_count_of_edge_group(
            &self.node.session,
            self.node.handle,
            &group,
            part_id,
        )
    }
    /// Get num geometry elements by type (points, prims, vertices).
    pub fn get_element_count_by_owner(
        &self,
        part: &PartInfo,
        owner: AttributeOwner,
    ) -> Result<i32> {
        crate::ffi::get_element_count_by_attribute_owner(part, owner)
    }

    /// Get number of attributes by type.
    pub fn get_attribute_count_by_owner(
        &self,
        part: &PartInfo,
        owner: AttributeOwner,
    ) -> Result<i32> {
        crate::ffi::get_attribute_count_by_owner(part, owner)
    }

    pub fn get_attribute_names(
        &self,
        owner: AttributeOwner,
        part: &PartInfo,
    ) -> Result<StringArray> {
        self.assert_node_cooked();
        let counts = part.attribute_counts();
        let count = match owner {
            AttributeOwner::Invalid => panic!("Invalid AttributeOwner"),
            AttributeOwner::Vertex => counts[0],
            AttributeOwner::Point => counts[1],
            AttributeOwner::Prim => counts[2],
            AttributeOwner::Detail => counts[3],
            AttributeOwner::Max => panic!("Invalid AttributeOwner"),
        };
        crate::ffi::get_attribute_names(&self.node, part.part_id(), count, owner)
    }

    /// Convenient method for getting the P attribute
    pub fn get_position_attribute(&self, part_id: i32) -> Result<NumericAttr<f32>> {
        self.assert_node_cooked();
        let name = CString::from(AttributeName::P);
        let info = AttributeInfo::new(&self.node, part_id, AttributeOwner::Point, name.as_c_str())?;
        Ok(NumericAttr::new(name, info, self.node.clone()))
    }

    /// Retrieve information about a geometry attribute.
    pub fn get_attribute_info(
        &self,
        part_id: i32,
        owner: AttributeOwner,
        name: impl TryInto<AttributeName, Error = impl Into<crate::HapiError>>,
    ) -> Result<AttributeInfo> {
        let name: AttributeName = name.try_into().map_err(Into::into)?;
        let name: CString = name.into();
        AttributeInfo::new(&self.node, part_id, owner, &name)
    }

    /// Get geometry attribute by name and owner.
    pub fn get_attribute<T>(
        &self,
        part_id: i32,
        owner: AttributeOwner,
        name: T,
    ) -> Result<Option<Attribute>>
    where
        T: TryInto<AttributeName>,
        T::Error: Into<crate::HapiError>,
    {
        self.assert_node_cooked();
        let name: AttributeName = name.try_into().map_err(Into::into)?;
        let name: CString = name.into();
        let info = AttributeInfo::new(&self.node, part_id, owner, &name)?;
        let storage = info.storage();
        if !info.exists() {
            return Ok(None);
        }
        let node = self.node.clone();
        let attr_obj: Box<dyn AnyAttribWrapper> = match storage {
            s @ (StorageType::Invalid | StorageType::Max) => {
                panic!("Invalid attribute storage {name:?}: {s:?}")
            }
            StorageType::Int => NumericAttr::<i32>::new(name, info, node).boxed(),
            StorageType::Int64 => NumericAttr::<i64>::new(name, info, node).boxed(),
            StorageType::Float => NumericAttr::<f32>::new(name, info, node).boxed(),
            StorageType::Float64 => NumericAttr::<f64>::new(name, info, node).boxed(),
            StorageType::String => StringAttr::new(name, info, node).boxed(),
            StorageType::Uint8 => NumericAttr::<u8>::new(name, info, node).boxed(),
            StorageType::Int8 => NumericAttr::<i8>::new(name, info, node).boxed(),
            StorageType::Int16 => NumericAttr::<i16>::new(name, info, node).boxed(),
            StorageType::IntArray => NumericArrayAttr::<i32>::new(name, info, node).boxed(),
            StorageType::Int64Array => NumericArrayAttr::<i64>::new(name, info, node).boxed(),
            StorageType::FloatArray => NumericArrayAttr::<f32>::new(name, info, node).boxed(),
            StorageType::Float64Array => NumericArrayAttr::<f64>::new(name, info, node).boxed(),
            StorageType::StringArray => StringArrayAttr::new(name, info, node).boxed(),
            StorageType::Uint8Array => NumericArrayAttr::<u8>::new(name, info, node).boxed(),
            StorageType::Int8Array => NumericArrayAttr::<i8>::new(name, info, node).boxed(),
            StorageType::Int16Array => NumericArrayAttr::<i16>::new(name, info, node).boxed(),
            StorageType::Dictionary => DictionaryAttr::new(name, info, node).boxed(),
            StorageType::DictionaryArray => DictionaryArrayAttr::new(name, info, node).boxed(),
        };
        Ok(Some(Attribute::new(attr_obj)))
    }

    /// Add a new numeric attribute to geometry.
    pub fn add_numeric_attribute<T: AttribAccess>(
        &self,
        name: &str,
        part_id: i32,
        info: AttributeInfo,
    ) -> Result<NumericAttr<T>> {
        debug_assert_eq!(info.storage(), T::storage());
        debug_assert!(
            info.tuple_size() > 0,
            "attribute \"{}\" tuple_size must be > 0",
            name
        );
        log::debug!("Adding numeric geometry attriubute: {name}");
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.0)?;
        Ok(NumericAttr::<T>::new(name, info, self.node.clone()))
    }

    /// Add a new numeric array attribute to geometry.
    pub fn add_numeric_array_attribute<T>(
        &self,
        name: &str,
        part_id: i32,
        info: AttributeInfo,
    ) -> Result<NumericArrayAttr<T>>
    where
        T: AttribAccess,
        [T]: ToOwned<Owned = Vec<T>>,
    {
        debug_assert_eq!(info.storage(), T::storage_array());
        debug_assert!(
            info.tuple_size() > 0,
            "AttributeInfo::tuple_size must be 1 for array attributes"
        );
        log::debug!("Adding numeric array geometry attriubute: {name}");
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.0)?;
        Ok(NumericArrayAttr::<T>::new(name, info, self.node.clone()))
    }

    /// Add a new string attribute to geometry
    pub fn add_string_attribute(
        &self,
        name: &str,
        part_id: i32,
        info: AttributeInfo,
    ) -> Result<StringAttr> {
        debug_assert!(self.node.is_valid()?);
        debug_assert_eq!(info.storage(), StorageType::String);
        debug_assert!(
            info.tuple_size() > 0,
            "attribute \"{}\" tuple_size must be > 0",
            name
        );
        log::debug!("Adding string geometry attriubute: {name}");
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.0)?;
        Ok(StringAttr::new(name, info, self.node.clone()))
    }

    /// Add a new string array attribute to geometry.
    pub fn add_string_array_attribute(
        &self,
        name: &str,
        part_id: i32,
        info: AttributeInfo,
    ) -> Result<StringArrayAttr> {
        debug_assert!(self.node.is_valid()?);
        debug_assert_eq!(info.storage(), StorageType::StringArray);
        debug_assert!(
            info.tuple_size() > 0,
            "attribute \"{}\" tuple_size must be > 0",
            name
        );
        log::debug!("Adding string array geometry attriubute: {name}");
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.0)?;
        Ok(StringArrayAttr::new(name, info, self.node.clone()))
    }

    /// Add a new dictionary attribute to geometry
    pub fn add_dictionary_attribute(
        &self,
        name: &str,
        part_id: i32,
        info: AttributeInfo,
    ) -> Result<DictionaryAttr> {
        debug_assert!(self.node.is_valid()?);
        debug_assert_eq!(info.storage(), StorageType::Dictionary);
        debug_assert!(
            info.tuple_size() > 0,
            "attribute \"{}\" tuple_size must be > 0",
            name
        );
        log::debug!("Adding dictionary geometry attriubute: {name}");
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.0)?;
        Ok(DictionaryAttr::new(name, info, self.node.clone()))
    }

    /// Add a new dictionary attribute to geometry
    pub fn add_dictionary_array_attribute(
        &self,
        name: &str,
        part_id: i32,
        info: AttributeInfo,
    ) -> Result<DictionaryArrayAttr> {
        debug_assert!(self.node.is_valid()?);
        debug_assert_eq!(info.storage(), StorageType::DictionaryArray);
        debug_assert!(
            info.tuple_size() > 0,
            "attribute \"{}\" tuple_size must be > 0",
            name
        );
        log::debug!("Adding dictionary array geometry attriubute: {name}");
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.0)?;
        Ok(DictionaryArrayAttr::new(name, info, self.node.clone()))
    }

    /// Create a new geometry group.
    pub fn add_group(
        &self,
        part_id: i32,
        group_type: GroupType,
        group_name: &str,
        membership: Option<&[i32]>,
    ) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        let group_name = CString::new(group_name)?;
        crate::ffi::add_group(
            &self.node.session,
            self.node.handle,
            part_id,
            group_type,
            &group_name,
        )?;
        match membership {
            None => Ok(()),
            Some(array) => crate::ffi::set_group_membership(
                &self.node.session,
                self.node.handle,
                part_id,
                group_type,
                &group_name,
                array,
            ),
        }
    }

    /// Delete a geometry group.
    pub fn delete_group(
        &self,
        part_id: i32,
        group_type: GroupType,
        group_name: &str,
    ) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        let group_name = CString::new(group_name)?;
        crate::ffi::delete_group(
            &self.node.session,
            self.node.handle,
            part_id,
            group_type,
            &group_name,
        )
    }

    /// Set element membership for a group.
    pub fn set_group_membership(
        &self,
        part_id: i32,
        group_type: GroupType,
        group_name: &str,
        array: &[i32],
    ) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        let group_name = CString::new(group_name)?;
        crate::ffi::set_group_membership(
            &self.node.session,
            self.node.handle,
            part_id,
            group_type,
            &group_name,
            array,
        )
    }

    /// Get element membership for a group.
    pub fn get_group_membership(
        &self,
        part: &PartInfo,
        group_type: GroupType,
        group_name: &str,
    ) -> Result<Vec<i32>> {
        self.assert_node_cooked();
        let group_name = CString::new(group_name)?;
        crate::ffi::get_group_membership(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            group_type,
            &group_name,
            part.element_count_by_group(group_type),
        )
    }

    /// Number of geometry groups by type.
    pub fn group_count_by_type(&self, group_type: GroupType) -> Result<i32> {
        self.assert_node_cooked();
        Ok(crate::ffi::get_group_count_by_type(&self.info, group_type))
    }

    pub fn get_instanced_part_ids(&self, part: &PartInfo) -> Result<Vec<i32>> {
        self.assert_node_cooked();
        crate::ffi::get_instanced_part_ids(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            part.instanced_part_count(),
        )
    }

    /// Get group membership for a packed instance part.
    /// This functions allows you to get the group membership for a specific packed primitive part.
    pub fn get_group_membership_on_packed_instance_part(
        &self,
        part: &PartInfo,
        group_type: GroupType,
        group_name: &CStr,
    ) -> Result<(bool, Vec<i32>)> {
        crate::ffi::get_group_membership_on_packed_instance_part(
            &self.node, part, group_type, group_name,
        )
    }

    pub fn get_group_count_on_packed_instance(&self, part: &PartInfo) -> Result<(i32, i32)> {
        self.assert_node_cooked();
        crate::ffi::get_group_count_on_instance_part(
            &self.node.session,
            self.node.handle,
            part.part_id(),
        )
    }

    pub fn get_instance_part_groups_names(
        &self,
        group: GroupType,
        part_id: i32,
    ) -> Result<StringArray> {
        self.assert_node_cooked();
        crate::ffi::get_group_names_on_instance_part(
            &self.node.session,
            self.node.handle,
            part_id,
            group,
        )
    }

    pub fn get_instance_part_transforms(
        &self,
        part: &PartInfo,
        order: RSTOrder,
    ) -> Result<Vec<Transform>> {
        self.assert_node_cooked();
        crate::ffi::get_instanced_part_transforms(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            order,
            part.instance_count(),
        )
        .map(|vec| vec.into_iter().map(Transform).collect())
    }

    /// Save geometry to a file.
    pub fn save_to_file(&self, filepath: &str) -> Result<()> {
        self.assert_node_cooked();
        let path = CString::new(filepath)?;
        crate::ffi::save_geo_to_file(&self.node, &path)
    }

    /// Load geometry from a file.
    pub fn load_from_file(&self, filepath: &str) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        let path = CString::new(filepath)?;
        crate::ffi::load_geo_from_file(&self.node, &path)
    }

    /// Commit geometry edits to the node.
    pub fn commit(&self) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        log::debug!("Commiting geometry changes");
        crate::ffi::commit_geo(&self.node)
    }

    /// Revert last geometry edits
    pub fn revert(&self) -> Result<()> {
        debug_assert!(self.node.is_valid()?);
        crate::ffi::revert_geo(&self.node)
    }

    /// Serialize node's geometry to bytes.
    pub fn save_to_memory(&self, format: GeoFormat) -> Result<Vec<i8>> {
        self.assert_node_cooked();
        crate::ffi::save_geo_to_memory(&self.node.session, self.node.handle, format.as_cstr())
    }

    /// Load geometry from a given buffer into this node.
    pub fn load_from_memory(&self, data: &[i8], format: GeoFormat) -> Result<()> {
        crate::ffi::load_geo_from_memory(
            &self.node.session,
            self.node.handle,
            data,
            format.as_cstr(),
        )
    }

    pub fn read_volume_tile<T: VolumeStorage>(
        &self,
        part: i32,
        fill: T,
        tile: &VolumeTileInfo,
        values: &mut [T],
    ) -> Result<()> {
        self.assert_node_cooked();
        T::read_tile(&self.node, part, fill, values, &tile.0)
    }

    pub fn write_volume_tile<T: VolumeStorage>(
        &self,
        part: i32,
        tile: &VolumeTileInfo,
        values: &[T],
    ) -> Result<()> {
        self.assert_node_cooked();
        T::write_tile(&self.node, part, values, &tile.0)
    }

    pub fn read_volume_voxel<T: VolumeStorage>(
        &self,
        part: i32,
        x_index: i32,
        y_index: i32,
        z_index: i32,
        values: &mut [T],
    ) -> Result<()> {
        self.assert_node_cooked();
        T::read_voxel(&self.node, part, x_index, y_index, z_index, values)
    }

    pub fn write_volume_voxel<T: VolumeStorage>(
        &self,
        part: i32,
        x_index: i32,
        y_index: i32,
        z_index: i32,
        values: &[T],
    ) -> Result<()> {
        self.assert_node_cooked();
        T::write_voxel(&self.node, part, x_index, y_index, z_index, values)
    }

    /// Iterate over volume tiles and apply a function to each tile.
    pub fn foreach_volume_tile(
        &self,
        part: i32,
        info: &VolumeInfo,
        callback: impl Fn(Tile),
    ) -> Result<()> {
        self.assert_node_cooked();
        let tile_size = (info.tile_size().pow(3) * info.tuple_size()) as usize;
        crate::volume::iterate_tiles(&self.node, part, tile_size, callback)
    }

    pub fn get_heightfield_data(&self, part_id: i32, volume_info: &VolumeInfo) -> Result<Vec<f32>> {
        self.assert_node_cooked();
        let length = volume_info.x_length() * volume_info.y_length();
        crate::ffi::get_heightfield_data(&self.node, part_id, length)
    }

    pub fn set_heightfield_data(&self, part_id: i32, name: &str, data: &[f32]) -> Result<()> {
        crate::ffi::set_heightfield_data(&self.node, part_id, &CString::new(name)?, data)
    }

    pub fn create_heightfield_input(
        &self,
        parent: impl Into<Option<NodeHandle>>,
        volume_name: &str,
        x_size: i32,
        y_size: i32,
        voxel_size: f32,
        sampling: HeightFieldSampling,
    ) -> Result<HeightfieldNodes> {
        let name = CString::new(volume_name)?;
        let (heightfield, height, mask, merge) = crate::ffi::create_heightfield_input(
            &self.node,
            parent.into(),
            &name,
            x_size,
            y_size,
            voxel_size,
            sampling,
        )?;
        Ok(HeightfieldNodes {
            heightfield: NodeHandle(heightfield).to_node(&self.node.session)?,
            height: NodeHandle(height).to_node(&self.node.session)?,
            mask: NodeHandle(mask).to_node(&self.node.session)?,
            merge: NodeHandle(merge).to_node(&self.node.session)?,
        })
    }

    pub fn create_heightfield_input_volume(
        &self,
        parent: impl Into<Option<NodeHandle>>,
        volume_name: &str,
        x_size: i32,
        y_size: i32,
        voxel_size: f32,
    ) -> Result<HoudiniNode> {
        let name = CString::new(volume_name)?;
        let handle = crate::ffi::create_heightfield_input_volume(
            &self.node,
            parent.into(),
            &name,
            x_size,
            y_size,
            voxel_size,
        )?;
        handle.to_node(&self.node.session)
    }
}

/// Holds HoudiniNode handles to a heightfield SOP
/// Used with [`Geometry::create_heightfield_input`]
pub struct HeightfieldNodes {
    pub heightfield: HoudiniNode,
    pub height: HoudiniNode,
    pub mask: HoudiniNode,
    pub merge: HoudiniNode,
}

impl PartInfo {
    pub fn element_count_by_group(&self, group_type: GroupType) -> i32 {
        crate::ffi::get_element_count_by_group(self, group_type)
    }
}

/// Geometry extension trait with some useful utilities
pub mod extra {
    use super::*;
    pub trait GeometryExtension {
        fn create_position_attribute(&self, part: &PartInfo) -> Result<NumericAttr<f32>>;
        fn create_point_color_attribute(&self, part: &PartInfo) -> Result<NumericAttr<f32>>;
        fn get_color_attribute(
            &self,
            part: &PartInfo,
            owner: AttributeOwner,
        ) -> Result<Option<NumericAttr<f32>>>;
        fn get_normal_attribute(
            &self,
            part: &PartInfo,
            owner: AttributeOwner,
        ) -> Result<Option<NumericAttr<f32>>>;
        fn get_position_attribute(&self, part: &PartInfo) -> Result<Option<NumericAttr<f32>>>;
    }

    impl GeometryExtension for Geometry {
        fn create_position_attribute(&self, part: &PartInfo) -> Result<NumericAttr<f32>> {
            create_point_tuple_attribute::<3>(self, part, AttributeName::P)
        }

        fn create_point_color_attribute(&self, part: &PartInfo) -> Result<NumericAttr<f32>> {
            create_point_tuple_attribute::<3>(self, part, AttributeName::Cd)
        }

        fn get_color_attribute(
            &self,
            part: &PartInfo,
            owner: AttributeOwner,
        ) -> Result<Option<NumericAttr<f32>>> {
            debug_assert!(matches!(
                owner,
                AttributeOwner::Point | AttributeOwner::Vertex
            ));
            get_tuple3_attribute(self, part, AttributeName::Cd, owner)
        }
        fn get_normal_attribute(
            &self,
            part: &PartInfo,
            owner: AttributeOwner,
        ) -> Result<Option<NumericAttr<f32>>> {
            debug_assert!(matches!(
                owner,
                AttributeOwner::Point | AttributeOwner::Vertex
            ));
            get_tuple3_attribute(self, part, AttributeName::N, owner)
        }
        fn get_position_attribute(&self, part: &PartInfo) -> Result<Option<NumericAttr<f32>>> {
            get_tuple3_attribute(self, part, AttributeName::P, AttributeOwner::Point)
        }
    }

    #[inline]
    fn create_point_tuple_attribute<const N: usize>(
        geo: &Geometry,
        part: &PartInfo,
        name: AttributeName,
    ) -> Result<NumericAttr<f32>> {
        log::debug!("Creating point attriute {:?}", name);
        let name: CString = name.try_into()?;
        let attr_info = AttributeInfo::default()
            .with_count(part.point_count())
            .with_tuple_size(N as i32)
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::Float);
        crate::ffi::add_attribute(&geo.node, part.part_id(), &name, &attr_info.0)
            .map(|_| NumericAttr::new(name, attr_info, geo.node.clone()))
    }

    #[inline]
    fn get_tuple3_attribute<'a>(
        geo: &'a Geometry,
        part: &PartInfo,
        name: AttributeName,
        owner: AttributeOwner,
    ) -> Result<Option<NumericAttr<f32>>> {
        let name: CString = name.try_into()?;
        AttributeInfo::new(&geo.node, part.part_id(), owner, &name).map(|info| {
            info.exists()
                .then(|| NumericAttr::new(name, info, geo.node.clone()))
        })
    }
}
