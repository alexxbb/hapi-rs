use crate::errors::Result;
use crate::ffi::raw::{AttributeOwner, StorageType};
pub use crate::ffi::AttributeInfo;
use crate::node::HoudiniNode;
use crate::session::Session;

#[derive(Debug)]
pub struct Int32Attribute<'s> {
    pub(crate) info: AttributeInfo<'s>,
    pub(crate) node: &'s HoudiniNode,
}

#[derive(Debug)]
pub struct Float32Attribute<'s> {
    pub(crate) info: AttributeInfo<'s>,
    pub(crate) node: &'s HoudiniNode,
}

#[derive(Debug)]
pub struct Int64Attribute {}
#[derive(Debug)]
pub struct Float64Attribute {}

impl Float32Attribute<'_> {
    pub fn get_values(&self, part_id: i32) -> Result<Vec<f32>> {
        crate::ffi::get_attribute_float_data(
            &self.node,
            part_id,
            &self.info.name,
            &self.info.inner,
            -1,
            0,
            self.info.count(),
        )
    }
    pub fn set_values(&self, values: impl AsRef<[f32]>) -> Result<()> {
        todo!()
    }
}

#[derive(Debug)]
pub enum Attribute<'session> {
    Int(Int32Attribute<'session>),
    Int64(Int64Attribute),
    Float(Float32Attribute<'session>),
    Float64(Float64Attribute),
}

impl<'s> Attribute<'s> {
    pub fn name(&self) -> &str {
        match self {
            Attribute::Int(a) => unimplemented!(),
            Attribute::Int64(a) => unimplemented!(),
            Attribute::Float(a) => a.info.name.to_str().expect("Non UTF attrib name").as_ref(),
            Attribute::Float64(a) => unimplemented!(),
        }
    }
}
