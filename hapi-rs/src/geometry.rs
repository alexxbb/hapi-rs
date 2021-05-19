use std::borrow::Cow;

pub use crate::attribute::*;
use crate::errors::Result;
pub use crate::ffi::{
    raw::{AttributeOwner, GroupType},
    GeoInfo,
};
use crate::ffi::{AttributeInfo, PartInfo};
use crate::node::HoudiniNode;
use crate::stringhandle::StringsArray;
use std::ffi::CString;

#[derive(Debug)]
pub struct Geometry<'session> {
    pub node: Cow<'session, HoudiniNode>,
    // TODO: Maybe revisit. GeoInfo may change and should be a get method
    pub info: GeoInfo<'session>,
}

impl<'session> Geometry<'session> {
    pub fn part_info(&'session self, id: i32) -> Result<PartInfo<'session>> {
        crate::ffi::get_part_info(&self.node, id).map(|inner| PartInfo {
            inner,
            session: &self.node.session,
        })
    }

    pub fn geo_info(&'session self) -> Result<GeoInfo<'session>> {
        crate::ffi::get_geo_info(&self.node).map(|inner| GeoInfo {
            inner,
            session: &self.node.session,
        })
    }

    pub fn get_face_counts(&self, info: &PartInfo) -> Result<Vec<i32>> {
        crate::ffi::get_face_counts(&self.node, info.part_id(), info.face_count())
    }

    pub fn get_group_names(&self, group_type: GroupType) -> Result<StringsArray> {
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
    ) -> Result<StringsArray> {
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
        let name = std::ffi::CString::new(name)?;
        let inner = crate::ffi::get_attribute_info(&self.node, part_id, owner, &name)?;
        if inner.exists < 1 {
            return Ok(None);
        }
        let attrib = Attribute::new(name, AttributeInfo { inner }, &self.node);
        Ok(Some(attrib))
    }

    pub fn add_attribute<T: AttribDataType>(
        &self,
        part_id: i32,
        name: &str,
        info: &AttributeInfo,
    ) -> Result<Attribute<T>> {
        let name = CString::new(name)?;
        crate::ffi::add_attribute(&self.node, part_id, &name, &info.inner)?;
        Ok(Attribute::new(name, AttributeInfo { inner: info.inner }, &self.node))
    }
}
