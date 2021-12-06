use std::borrow::Cow;

pub use crate::attribute::*;
use crate::errors::Result;
pub use crate::ffi::{
    raw::{AttributeOwner, CurveOrders, CurveType, GroupType, PackedPrimInstancingMode, PartType},
    AttributeInfo, CurveInfo, GeoInfo, PartInfo,
};
use crate::node::HoudiniNode;
use crate::stringhandle::StringArray;
use std::ffi::CString;

#[derive(Debug)]
pub struct Geometry {
    pub node: HoudiniNode,
    pub(crate) info: GeoInfo,
}

impl Geometry {
    pub fn part_info(&self, id: i32) -> Result<PartInfo> {
        crate::ffi::get_part_info(&self.node, id).map(|inner| PartInfo { inner })
    }

    pub fn set_part_info(&self, info: &PartInfo) -> Result<()> {
        // TODO: Should part_id be provided by user or by PartInfo?
        crate::ffi::set_part_info(&self.node, info)
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

    pub fn geo_info(&self) -> Result<GeoInfo> {
        GeoInfo::from_node(&self.node)
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
    pub fn vertex_list(&self, part: &PartInfo) -> Result<Vec<i32>> {
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

    pub fn get_face_counts(&self, part: &PartInfo) -> Result<Vec<i32>> {
        crate::ffi::get_face_counts(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            0,
            part.face_count(),
        )
    }

    pub fn get_group_names(&self, group_type: GroupType) -> Result<StringArray> {
        let count = match group_type {
            GroupType::Point => self.info.point_group_count(),
            GroupType::Prim => self.info.primitive_group_count(),
            _ => unreachable!("Impossible GroupType value"),
        };
        crate::ffi::get_group_names(&self.node, group_type, count)
    }

    pub fn get_attribute_names(
        &self,
        owner: AttributeOwner,
        part: &PartInfo,
    ) -> Result<StringArray> {
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

    pub fn get_attribute<T: AttribDataType>(
        &self,
        part_id: i32,
        owner: AttributeOwner,
        name: &str,
    ) -> Result<Option<Attribute<T>>> {
        let _n = name;
        let name = std::ffi::CString::new(name)?;
        let inner = crate::ffi::get_attribute_info(&self.node, part_id, owner, &name)?;

        if inner.storage != T::storage() {
            return Ok(None);
        }
        if inner.exists < 1 {
            return Ok(None);
        }
        let attrib = Attribute::new(name, AttributeInfo { inner }, &self.node);
        Ok(Some(attrib))
    }

    pub fn add_attribute<T: AttribDataType>(
        &self,
        name: &str,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Attribute<T>> {
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.inner)?;
        Ok(Attribute::new(
            name,
            AttributeInfo { inner: info.inner },
            &self.node,
        ))
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
        let group_name = CString::new(group_name)?;
        let tmp;
        let part = match part_info {
            None => {
                tmp = self.part_info(0)?;
                &tmp
            }
            Some(part) => part,
        };
        crate::ffi::get_group_membership(
            &self.node.session,
            self.node.handle,
            part.part_id(),
            group_type,
            &group_name,
            part.element_count_by_group(group_type),
        )
    }

    pub fn group_count_by_type(&self, group_type: GroupType) -> i32 {
        crate::ffi::get_group_count_by_type(&self.info, group_type)
    }

    pub fn save_to_file(&self, filepath: &str) -> Result<()> {
        let path = CString::new(filepath)?;
        crate::ffi::save_geo_to_file(&self.node, &path)
    }

    pub fn commit(&self) -> Result<()> {
        crate::ffi::commit_geo(&self.node)
    }
}

impl PartInfo {
    pub fn element_count_by_group(&self, group_type: GroupType) -> i32 {
        crate::ffi::get_element_count_by_group(self, group_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::tests::with_session;
    use crate::session::Session;

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
            .add_attribute::<f32>("P", part.part_id(), &info)
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
        use crate::session::tests::OTLS;
        let lib = session
            .load_asset_file(OTLS.get("geometry").unwrap())
            .expect("Could not load otl");
        let node = lib.try_create_first()?;
        node.cook(None).unwrap();
        node.geometry()
            .map(|some| some.expect("must have geometry"))
    }

    #[test]
    fn incorrect_attributes() {
        with_session(|session| {
            let geo = _load_test_geometry(session).unwrap();
            let foo_bar = geo
                .get_attribute::<i64>(0, AttributeOwner::Prim, "foo_bar")
                .expect("attribute");
            assert!(foo_bar.is_none());
            let pscale = geo
                .get_attribute::<i64>(0, AttributeOwner::Point, "pscale")
                .expect("attribute");
            assert!(pscale.is_none(), "pscale type is f32");
        });
    }

    #[test]
    fn numeric_attributes() {
        with_session(|session| {
            let input = session.create_input_node("test").unwrap();
            let geo = _create_triangle(&input);
            let attr_p = geo
                .get_attribute::<f32>(0, AttributeOwner::Point, "P")
                .unwrap()
                .unwrap();
            let val: Vec<_> = attr_p.read(0).expect("read_attribute");
            assert_eq!(val.len(), 9);
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

            let attr_name = geo.add_attribute::<&str>("name", 0, &info).unwrap();
            attr_name.set(0, ["pt0", "pt1", "pt2"]).unwrap();
            geo.commit().unwrap();
        });
    }

    #[test]
    fn array_attributes() {
        use crate::session::tests::OTLS;
        with_session(|session| {
            let geo = _load_test_geometry(session).expect("geometry");

            let attr = geo
                .get_attribute::<i32>(0, AttributeOwner::Point, "my_int_array")
                .expect("attribute")
                .unwrap();
            let i_array = attr.read_array(0).unwrap();

            assert_eq!(i_array.iter().count(), attr.info.count() as usize);
            assert_eq!(i_array.iter().nth(0).unwrap(), &[0, 0, 0, -1]);
            assert_eq!(i_array.iter().last().unwrap(), &[7, 14, 21, -1]);

            let attr = geo
                .get_attribute::<f32>(0, AttributeOwner::Point, "my_float_array")
                .expect("attribute")
                .unwrap();
            let i_array = attr.read_array(0).unwrap();

            assert_eq!(i_array.iter().count(), attr.info.count() as usize);
            assert_eq!(i_array.iter().nth(0).unwrap(), &[0.0, 0.0, 0.0]);
            assert_eq!(i_array.iter().last().unwrap(), &[7.0, 14.0, 21.0]);
        });
    }

    #[test]
    fn string_array_attribute() {
        with_session(|session| {
            let geo = _load_test_geometry(session).expect("geometry");
            let attr = geo
                .get_attribute::<&str>(0, AttributeOwner::Point, "my_str_array")
                .expect("attribute")
                .unwrap();
            let i_array = attr.read_array(0).unwrap();
            assert_eq!(i_array.iter().count(), attr.info.count() as usize);

            let it = i_array.iter().nth(0).unwrap().unwrap();
            let pt_0: Vec<&str> = it.iter_str().collect();
            assert_eq!(pt_0, ["pt_0_0", "pt_0_1", "pt_0_2", "start"]);

            let it = i_array.iter().nth(1).unwrap().unwrap();
            let pt_1: Vec<&str> = it.iter_str().collect();
            assert_eq!(pt_1, ["pt_1_0", "pt_1_1", "pt_1_2"]);

            let it = i_array.iter().last().unwrap().unwrap();
            let pt_n: Vec<&str> = it.iter_str().collect();
            assert_eq!(pt_n, ["pt_7_0", "pt_7_1", "pt_7_2", "end"]);
        });
    }
}
