use std::ffi::CStr;
use std::marker::PhantomData;

use crate::errors::Result;
use crate::ffi::raw::{AttributeOwner, StorageType};
pub use crate::ffi::AttributeInfo;
use crate::node::HoudiniNode;
use crate::session::Session;

pub trait AttribDataType: Sized {
    fn read(node: &HoudiniNode, part_id: i32, info: &AttributeInfo) -> Result<Vec<Self>>;
    fn set(values: impl AsRef<[Self]>) -> Result<()>;
}

pub struct Attribute<'s, T: AttribDataType> {
    pub(crate) info: AttributeInfo<'s>,
    pub(crate) node: &'s HoudiniNode,
    _marker: PhantomData<T>,
}

impl<'s, T: AttribDataType> Attribute<'s, T> {
    pub(crate) fn new(info: AttributeInfo<'s>, node: &'s HoudiniNode) -> Self {
        Attribute::<T> {
            info,
            node,
            _marker: Default::default(),
        }
    }
    pub fn read(&self, part_id: i32) -> Result<Vec<T>> {
        T::read(self.node, part_id, &self.info)
    }
}

impl AttribDataType for f32 {
    fn read<'session>(
        node: &HoudiniNode,
        part_id: i32,
        info: &AttributeInfo<'session>,
    ) -> Result<Vec<Self>> {
        crate::ffi::get_attribute_float_data(
            node,
            part_id,
            &info.name,
            &info.inner,
            -1,
            0,
            info.count(),
        )
    }

    fn set(values: impl AsRef<[Self]>) -> Result<()> {
        unimplemented!()
    }
}

impl AttribDataType for i32 {
    fn read<'session>(
        node: &HoudiniNode,
        part_id: i32,
        info: &AttributeInfo<'session>,
    ) -> Result<Vec<Self>> {
        crate::ffi::get_attribute_int_data(
            node,
            part_id,
            &info.name,
            &info.inner,
            -1,
            0,
            info.count(),
        )
    }

    fn set(values: impl AsRef<[Self]>) -> Result<()> {
        unimplemented!()
    }
}
