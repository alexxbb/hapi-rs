use std::borrow::Cow;

pub use crate::attribute::*;
use crate::errors::Result;
pub use crate::ffi::{raw::AttributeOwner, GeoInfo};
use crate::ffi::{AttributeInfo, PartInfo};
use crate::node::HoudiniNode;

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

    pub fn get_attribute_names(
        &self,
        owner: AttributeOwner,
        part: &PartInfo,
    ) -> Result<Vec<String>> {
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
    ) -> Result<Attribute<T>> {
        let name = std::ffi::CString::new(name)?;
        let inner = crate::ffi::get_attribute_info(&self.node, part_id, owner, &name)?;
        let info = AttributeInfo {
            name,
            inner,
            session: &self.node.session,
        };
        use crate::ffi::raw::StorageType;
        let attrib = Attribute::new(info, &self.node);
        Ok(attrib)
    }
}
