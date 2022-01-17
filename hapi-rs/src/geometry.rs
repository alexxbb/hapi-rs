#![allow(unused)]
use std::ffi::{CStr, CString};

use crate::attribute::*;
use crate::errors::Result;
pub use crate::ffi::{
    enums::*, AttributeInfo, BoxInfo, CookOptions, CurveInfo, GeoInfo, PartInfo, Transform,
};
use crate::material::Material;
use crate::node::{HoudiniNode, NodeHandle};
use crate::session::Session;
use crate::stringhandle::StringArray;

macro_rules! unwrap_or_create {
    ($out:ident, $opt:expr, $default:expr) => {
        match $opt {
            None => {
                $out = $default;
                &$out
            }
            Some(v) => v,
        }
    };
}

#[derive(Debug, Clone)]
pub struct Geometry {
    pub node: HoudiniNode,
    pub(crate) info: GeoInfo,
}

#[derive(Debug)]
pub enum GeoFormat {
    Geo,
    Bgeo,
    Obj,
}

#[derive(Debug)]
/// Single - when material is assigned at object level
/// Multiple - when assigned per-face
pub enum Materials {
    Single(Material),
    Multiple(Vec<Material>),
}

impl GeoFormat {
    fn as_c_literal(&self) -> &'static [u8] {
        match *self {
            GeoFormat::Geo => b".geo\0",
            GeoFormat::Bgeo => b".bgeo\0",
            GeoFormat::Obj => b".obj\0",
        }
    }
}

impl Geometry {
    pub fn from_info(info: GeoInfo, session: &Session) -> Result<Self> {
        Ok(Self {
            node: info.node_id().to_node(session)?,
            info,
        })
    }

    pub fn part_info(&self, part_id: i32) -> Result<PartInfo> {
        crate::ffi::get_part_info(&self.node, part_id).map(|inner| PartInfo { inner })
    }

    pub fn geo_info(&self) -> Result<GeoInfo> {
        GeoInfo::from_node(&self.node)
    }

    pub fn set_part_info(&self, info: &PartInfo) -> Result<()> {
        crate::ffi::set_part_info(&self.node, info)
    }

    pub fn box_info(&self, part_id: i32) -> Result<BoxInfo> {
        crate::ffi::get_box_info(self.node.handle, &self.node.session, part_id)
            .map(|inner| BoxInfo { inner })
    }

    pub fn sphere_info(&self, part_id: i32) -> Result<BoxInfo> {
        crate::ffi::get_box_info(self.node.handle, &self.node.session, part_id)
            .map(|inner| BoxInfo { inner })
    }

    pub fn set_curve_info(&self, info: &CurveInfo, part_id: i32) -> Result<()> {
        crate::ffi::set_curve_info(&self.node, info, part_id)
    }

    pub fn set_curve_counts(&self, part_id: i32, count: &[i32]) -> Result<()> {
        crate::ffi::set_curve_counts(&self.node, part_id, count)
    }

    pub fn set_curve_knots(&self, part_id: i32, knots: &[f32]) -> Result<()> {
        crate::ffi::set_curve_knots(&self.node, part_id, knots)
    }

    pub fn set_vertex_list(&self, part_id: i32, list: impl AsRef<[i32]>) -> Result<()> {
        crate::ffi::set_geo_vertex_list(&self.node, part_id, list.as_ref())
    }

    pub fn set_face_counts(&self, part_id: i32, list: impl AsRef<[i32]>) -> Result<()> {
        crate::ffi::set_geo_face_counts(&self.node, part_id, list.as_ref())
    }

    pub fn update(&mut self) -> Result<()> {
        self.info = self.geo_info()?;
        Ok(())
    }

    pub fn curve_info(&self, part_id: i32) -> Result<CurveInfo> {
        crate::ffi::get_curve_info(&self.node, part_id).map(|inner| CurveInfo { inner })
    }

    /// Retrieve the number of vertices for each curve in the part.
    pub fn curve_counts(&self, part_id: i32, start: i32, length: i32) -> Result<Vec<i32>> {
        crate::ffi::get_curve_counts(&self.node, part_id, start, length)
    }

    /// Retrieve the orders for each curve in the part if the curve has varying order.
    pub fn curve_orders(&self, part_id: i32, start: i32, length: i32) -> Result<Vec<i32>> {
        crate::ffi::get_curve_orders(&self.node, part_id, start, length)
    }

    /// Retrieve the knots of the curves in this part.
    pub fn curve_knots(&self, part_id: i32, start: i32, length: i32) -> Result<Vec<f32>> {
        crate::ffi::get_curve_knots(&self.node, part_id, start, length)
    }

    /// Get array containing the vertex-point associations where the
    /// ith element in the array is the point index the ith vertex
    /// associates with.
    pub fn vertex_list(&self, part: Option<&PartInfo>) -> Result<Vec<i32>> {
        let tmp;
        let part = unwrap_or_create!(tmp, part, self.part_info(0)?);
        crate::ffi::get_geo_vertex_list(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            0,
            part.vertex_count(),
        )
    }

    pub fn partitions(&self) -> Result<Vec<PartInfo>> {
        #[cfg(debug_assertions)]
        if self.node.info.total_cook_count() == 0 {
            log::warn!("Node {} not cooked", self.node.path(None)?);
        }
        (0..self.info.part_count())
            .map(|i| self.part_info(i))
            .collect()
    }

    pub fn get_face_counts(&self, part: Option<&PartInfo>) -> Result<Vec<i32>> {
        let tmp;
        let part = unwrap_or_create!(tmp, part, self.part_info(0)?);
        crate::ffi::get_face_counts(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            0,
            part.face_count(),
        )
    }

    pub fn get_materials(&self, part: Option<&PartInfo>) -> Result<Option<Materials>> {
        let tmp;
        let part = unwrap_or_create!(tmp, part, self.part_info(0)?);
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
                let mat_node = NodeHandle(mats[0], ());
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
                    crate::ffi::get_material_info(&session, NodeHandle(id, ())).map(|info| {
                        Material {
                            session: session.clone(),
                            info,
                        }
                    })
                })
                .collect::<Result<Vec<_>>>();
            mats.map(|vec| Some(Materials::Multiple(vec)))
        }
    }

    pub fn get_group_names(&self, group_type: GroupType) -> Result<StringArray> {
        let count = match group_type {
            GroupType::Point => self.info.point_group_count(),
            GroupType::Prim => self.info.primitive_group_count(),
            GroupType::Edge => self.info.edge_group_count(),
            _ => unreachable!("Impossible GroupType value"),
        };
        crate::ffi::get_group_names(&self.node, group_type, count)
    }

    pub fn get_edge_count_of_edge_group(&self, group: &str, part_id: i32) -> Result<i32> {
        let group = CString::new(group)?;
        crate::ffi::get_edge_count_of_edge_group(
            &self.node.session,
            self.node.handle,
            &group,
            part_id,
        )
    }

    pub fn get_attribute_names(
        &self,
        owner: AttributeOwner,
        part: Option<&PartInfo>,
    ) -> Result<StringArray> {
        let tmp;
        let part = unwrap_or_create!(tmp, part, self.part_info(0)?);
        let counts = part.attribute_counts();
        let count = match owner {
            AttributeOwner::Invalid => panic!("Invalid AttributeOwner"),
            AttributeOwner::Vertex => counts[0],
            AttributeOwner::Point => counts[1],
            AttributeOwner::Prim => counts[2],
            AttributeOwner::Detail => counts[3],
            AttributeOwner::Max => unreachable!(),
        };
        crate::ffi::get_attribute_names(&self.node, part.part_id(), count, owner)
    }

    pub fn get_attribute(
        &self,
        part_id: i32,
        owner: AttributeOwner,
        name: &str,
    ) -> Result<Option<Attribute>> {
        let _n = name;
        let name = std::ffi::CString::new(name)?;
        let inner = crate::ffi::get_attribute_info(&self.node, part_id, owner, &name)?;
        let storage = inner.storage;
        if inner.exists < 1 {
            return Ok(None);
        }
        let info = AttributeInfo { inner };
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
            StorageType::Array => NumericArrayAttr::<i32>::new(name, info, node).boxed(),
            StorageType::Int64Array => NumericArrayAttr::<i64>::new(name, info, node).boxed(),
            StorageType::FloatArray => NumericArrayAttr::<f32>::new(name, info, node).boxed(),
            StorageType::Float64Array => NumericArrayAttr::<f64>::new(name, info, node).boxed(),
            StorageType::StringArray => StringArrayAttr::new(name, info, node).boxed(),
            StorageType::Uint8Array => NumericArrayAttr::<u8>::new(name, info, node).boxed(),
            StorageType::Int8Array => NumericArrayAttr::<i8>::new(name, info, node).boxed(),
            StorageType::Int16Array => NumericArrayAttr::<i16>::new(name, info, node).boxed(),
        };
        Ok(Some(Attribute::new(attr_obj)))
    }

    pub fn add_numeric_attribute<T: AttribAccess>(
        &self,
        name: &str,
        part_id: i32,
        info: AttributeInfo,
    ) -> Result<NumericAttr<T>> {
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.inner)?;
        Ok(NumericAttr::<T>::new(name, info, self.node.clone()))
    }

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
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.inner)?;
        Ok(NumericArrayAttr::<T>::new(name, info, self.node.clone()))
    }

    pub fn add_string_attribute(
        &self,
        name: &str,
        part_id: i32,
        info: AttributeInfo,
    ) -> Result<StringAttr> {
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.inner)?;
        Ok(StringAttr::new(name, info, self.node.clone()))
    }

    pub fn add_string_array_attribute(
        &self,
        name: &str,
        part_id: i32,
        info: AttributeInfo,
    ) -> Result<StringArrayAttr> {
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.inner)?;
        Ok(StringArrayAttr::new(name, info, self.node.clone()))
    }

    pub fn add_group(
        &self,
        part_id: i32,
        group_type: GroupType,
        group_name: &str,
        membership: Option<&[i32]>,
    ) -> Result<()> {
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

    pub fn delete_group(
        &self,
        part_id: i32,
        group_type: GroupType,
        group_name: &str,
    ) -> Result<()> {
        let group_name = CString::new(group_name)?;
        crate::ffi::delete_group(
            &self.node.session,
            self.node.handle,
            part_id,
            group_type,
            &group_name,
        )
    }

    pub fn set_group_membership(
        &self,
        part_id: i32,
        group_type: GroupType,
        group_name: &str,
        array: &[i32],
    ) -> Result<()> {
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

    pub fn get_group_membership(
        &self,
        part_info: Option<&PartInfo>,
        group_type: GroupType,
        group_name: &str,
    ) -> Result<Vec<i32>> {
        let tmp;
        let part = unwrap_or_create!(tmp, part_info, self.part_info(0)?);
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

    pub fn group_count_by_type(
        &self,
        group_type: GroupType,
        info: Option<&GeoInfo>,
    ) -> Result<i32> {
        let tmp;
        let info = unwrap_or_create!(tmp, info, self.geo_info()?);
        Ok(crate::ffi::get_group_count_by_type(info, group_type))
    }

    pub fn get_instanced_part_ids(&self, part_info: Option<&PartInfo>) -> Result<Vec<i32>> {
        let tmp;
        let part = unwrap_or_create!(tmp, part_info, self.part_info(0)?);
        crate::ffi::get_instanced_part_ids(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            part.instanced_part_count(),
        )
    }

    pub fn get_group_count_on_packed_instance(
        &self,
        part_info: Option<&PartInfo>,
    ) -> Result<(i32, i32)> {
        let tmp;
        let part = unwrap_or_create!(tmp, part_info, self.part_info(0)?);
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
        crate::ffi::get_group_names_on_instance_part(
            &self.node.session,
            self.node.handle,
            part_id,
            group,
        )
    }

    pub fn get_instance_part_transforms(
        &self,
        part_info: Option<&PartInfo>,
        order: RSTOrder,
    ) -> Result<Vec<Transform>> {
        let tmp;
        let part = unwrap_or_create!(tmp, part_info, self.part_info(0)?);
        crate::ffi::get_instanced_part_transforms(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            order,
            part.instance_count(),
        )
        .map(|vec| vec.into_iter().map(|inner| Transform { inner }).collect())
    }

    pub fn save_to_file(&self, filepath: &str) -> Result<()> {
        let path = CString::new(filepath)?;
        crate::ffi::save_geo_to_file(&self.node, &path)
    }

    pub fn load_from_file(&self, filepath: &str) -> Result<()> {
        let path = CString::new(filepath)?;
        crate::ffi::load_geo_from_file(&self.node, &path)
    }

    pub fn commit(&self) -> Result<()> {
        crate::ffi::commit_geo(&self.node)
    }

    pub fn revert(&self) -> Result<()> {
        crate::ffi::revert_geo(&self.node)
    }

    pub fn save_to_memory(&self, format: GeoFormat) -> Result<Vec<i8>> {
        let format = unsafe { CStr::from_bytes_with_nul_unchecked(format.as_c_literal()) };
        crate::ffi::save_geo_to_memory(&self.node.session, self.node.handle, format)
    }

    pub fn load_from_memory(&self, data: &[i8], format: GeoFormat) -> Result<()> {
        let format = unsafe { CStr::from_bytes_with_nul_unchecked(format.as_c_literal()) };
        crate::ffi::load_geo_from_memory(&self.node.session, self.node.handle, data, format)
    }
}

impl PartInfo {
    pub fn element_count_by_group(&self, group_type: GroupType) -> i32 {
        crate::ffi::get_element_count_by_group(self, group_type)
    }
}

#[cfg(test)]
mod tests {
    use crate::geometry::Geometry;
    use crate::node::HoudiniNode;
    use crate::session::tests::with_session;
    use crate::session::Session;

    use super::*;

    fn _create_triangle(node: &HoudiniNode) -> Geometry {
        let geo = node.geometry().expect("geometry").unwrap();

        let part = PartInfo::default()
            .with_part_type(PartType::Mesh)
            .with_face_count(1)
            .with_point_count(3)
            .with_vertex_count(3);
        geo.set_part_info(&part).expect("part_info");
        let info = AttributeInfo::default()
            .with_count(part.point_count())
            .with_tuple_size(3)
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::Float);
        let attr_p = geo
            .add_numeric_attribute::<f32>("P", part.part_id(), info)
            .unwrap();
        attr_p
            .set(
                part.part_id(),
                &[0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
            )
            .unwrap();
        geo.set_vertex_list(0, [0, 1, 2]).unwrap();
        geo.set_face_counts(0, [3]).unwrap();
        geo.commit().expect("commit");
        node.cook_blocking(None).unwrap();
        geo
    }

    fn _load_test_geometry(session: &Session) -> super::Result<Geometry> {
        let node = session.create_node("Object/hapi_geo", None, None)?;
        node.cook(None).unwrap();
        node.geometry()
            .map(|some| some.expect("must have geometry"))
    }

    #[test]
    fn wrong_attribute() {
        with_session(|session| {
            let geo = _load_test_geometry(session).unwrap();
            let foo_bar = geo
                .get_attribute(0, AttributeOwner::Prim, "foo_bar")
                .expect("attribute");
            assert!(foo_bar.is_none());
        });
    }

    #[test]
    fn numeric_attributes() {
        with_session(|session| {
            let input = session.create_input_node("test").unwrap();
            let geo = _create_triangle(&input);
            let attr_p = geo
                .get_attribute(0, AttributeOwner::Point, "P")
                .unwrap()
                .unwrap();
            let attr_p = attr_p.downcast::<NumericAttr<f32>>().unwrap();
            let dat = attr_p.get(0).expect("read_attribute");
            assert_eq!(dat.len(), 9);
            input.delete().unwrap();
        });
    }

    #[test]
    fn create_string_attrib() {
        with_session(|session| {
            let input = session.create_input_node("test").unwrap();
            let geo = _create_triangle(&input);
            let part = geo.part_info(0).unwrap();
            let info = AttributeInfo::default()
                .with_owner(AttributeOwner::Point)
                .with_storage(StorageType::String)
                .with_tuple_size(1)
                .with_count(part.point_count());

            let attr_name = geo.add_string_attribute("name", 0, info).unwrap();
            attr_name.set(0, &["pt0", "pt1", "pt2"]).unwrap();
            geo.commit().unwrap();
            input.delete().unwrap();
        });
    }

    #[test]
    fn array_attributes() {
        with_session(|session| {
            let geo = _load_test_geometry(session).expect("geometry");

            let attr = geo
                .get_attribute(0, AttributeOwner::Point, "my_int_array")
                .expect("attribute")
                .unwrap();
            let attr = attr.downcast::<NumericArrayAttr<i32>>().unwrap();
            let i_array = attr.get(0).unwrap();
            assert_eq!(i_array.iter().count(), attr.info().count() as usize);
            assert_eq!(i_array.iter().next().unwrap(), &[0, 0, 0, -1]);
            assert_eq!(i_array.iter().last().unwrap(), &[7, 14, 21, -1]);

            let attr = geo
                .get_attribute(0, AttributeOwner::Point, "my_float_array")
                .expect("attribute")
                .unwrap();
            let i_array = attr.downcast::<NumericArrayAttr<f32>>().unwrap();
            let data = i_array.get(0).unwrap();

            assert_eq!(data.iter().count(), attr.info().count() as usize);
            assert_eq!(data.iter().next().unwrap(), &[0.0, 0.0, 0.0]);
            assert_eq!(data.iter().last().unwrap(), &[7.0, 14.0, 21.0]);
        });
    }

    #[test]
    #[ignore]
    fn string_array_attribute() {
        with_session(|session| {
            let geo = _load_test_geometry(session).expect("geometry");
            let attr = geo
                .get_attribute(0, AttributeOwner::Point, "my_str_array")
                .expect("attribute")
                .unwrap();
            let attr = attr.downcast::<StringArrayAttr>().unwrap();
            let m_array = attr.get(0).unwrap();
            assert_eq!(m_array.iter().count(), attr.info().count() as usize);

            let it = m_array.iter().next().unwrap().unwrap();
            let pt_0: Vec<&str> = it.iter_str().collect();
            assert_eq!(pt_0, ["pt_0_0", "pt_0_1", "pt_0_2", "start"]);

            let it = m_array.iter().nth(1).unwrap().unwrap();
            let pt_1: Vec<&str> = it.iter_str().collect();
            assert_eq!(pt_1, ["pt_1_0", "pt_1_1", "pt_1_2"]);

            let it = m_array.iter().last().unwrap().unwrap();
            let pt_n: Vec<&str> = it.iter_str().collect();
            assert_eq!(pt_n, ["pt_7_0", "pt_7_1", "pt_7_2", "end"]);
        });
    }

    #[test]
    fn save_and_load_to_file() {
        with_session(|session| {
            let input = session.create_input_node("triangle").unwrap();
            let geo = _create_triangle(&input);
            let tmp_file = std::env::temp_dir().join("triangle.geo");
            geo.save_to_file(&tmp_file.to_string_lossy())
                .expect("save_to_file");
            let input = session.create_input_node("dummy").unwrap();
            let geo = input.geometry().unwrap().unwrap();
            geo.load_from_file(&tmp_file.to_string_lossy())
                .expect("load_from_file");
            geo.node.cook(None).unwrap();
            assert_eq!(geo.part_info(0).unwrap().point_count(), 3);
            input.delete().unwrap();
        });
    }

    #[test]
    fn geometry_in_memory() {
        with_session(|session| {
            let node = session.create_input_node("source").unwrap();
            let source = _create_triangle(&node);
            let blob = source
                .save_to_memory(super::GeoFormat::Geo)
                .expect("save_geo_to_memory");
            node.delete().unwrap();

            let node = session.create_input_node("dest").unwrap();
            let dest = _create_triangle(&node);
            dest.load_from_memory(&blob, super::GeoFormat::Geo)
                .expect("load_from_memory");
            node.delete().unwrap();
        });
    }

    #[test]
    fn commit_and_revert() {
        with_session(|session| {
            let input = session.create_input_node("input").unwrap();
            let geo = _create_triangle(&input);
            geo.commit().unwrap();
            input.cook_blocking(None).unwrap();
            assert_eq!(geo.part_info(0).unwrap().point_count(), 3);
            geo.revert().unwrap();
            input.cook_blocking(None).unwrap();
            assert_eq!(geo.part_info(0).unwrap().point_count(), 0);
            input.delete().unwrap();
        });
    }

    #[test]
    fn add_and_delete_group() {
        with_session(|session| {
            let input = session.create_input_node("input").unwrap();
            let geo = _create_triangle(&input);
            geo.add_group(0, GroupType::Point, "test", Some(&[1, 1, 1]))
                .unwrap();
            geo.commit().unwrap();
            geo.node.cook_blocking(None).unwrap();
            assert_eq!(
                geo.group_count_by_type(GroupType::Point, geo.geo_info().as_ref().ok()),
                Ok(1)
            );

            geo.delete_group(0, GroupType::Point, "test").unwrap();
            geo.commit().unwrap();
            geo.node.cook_blocking(None).unwrap();
            assert_eq!(geo.group_count_by_type(GroupType::Point, None), Ok(0));
            input.delete().unwrap();
        });
    }

    #[test]
    fn basic_instancing() {
        with_session(|session| {
            use crate::session::tests::OTLS;
            let lib = session
                .load_asset_file(OTLS.get("geometry").unwrap())
                .expect("Could not load otl");
            let node = lib.try_create_first().unwrap();
            let opt = CookOptions::default()
                .with_packed_prim_instancing_mode(PackedPrimInstancingMode::Flat);
            node.cook_blocking(Some(&opt)).unwrap();
            let outputs = node.geometry_outputs().unwrap();
            let instancer = outputs.get(1).unwrap();
            let ids = instancer.get_instanced_part_ids(None).unwrap();
            assert_eq!(ids.len(), 1);
            let names = instancer
                .get_instance_part_groups_names(GroupType::Prim, ids[0])
                .unwrap();
            let names: Vec<String> = names.into_iter().collect();
            assert_eq!(names.first().unwrap(), "group_1");
            assert_eq!(names.last().unwrap(), "group_6");
            let tranforms = instancer
                .get_instance_part_transforms(None, RSTOrder::Srt)
                .unwrap();
            assert_eq!(
                tranforms.len() as i32,
                instancer.part_info(0).unwrap().instance_count()
            );
        });
    }

    #[test]
    fn get_face_materials() {
        with_session(|session| {
            let node = session.create_node("Object/spaceship", None, None).unwrap();
            node.cook(None).unwrap();
            let geo = node.geometry().expect("geometry").unwrap();
            let mats = geo.get_materials(None).expect("materials");
            assert!(matches!(mats, Some(Materials::Single(_))));
        });
    }
}
