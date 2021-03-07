use std::ffi::CStr;
use std::marker::PhantomData;

use crate::errors::Result;
use crate::ffi::raw::{AttributeOwner, StorageType};
pub use crate::ffi::AttributeInfo;
use crate::node::HoudiniNode;
use crate::session::Session;
use crate::stringhandle::StringsArray;

pub trait AttribDataType: Sized {
    type Type;
    type Return;
    fn read(
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &'_ AttributeInfo<'_>,
    ) -> Result<Self::Return>;
    fn set(values: impl AsRef<[Self::Type]>) -> Result<()>;
}

#[derive(Debug)]
pub struct Attribute<'s, T: AttribDataType> {
    pub(crate) info: AttributeInfo<'s>,
    pub(crate) node: &'s HoudiniNode,
    _marker: PhantomData<T>,
}

impl<'s, T> Attribute<'s, T>
where
    T: AttribDataType,
{
    pub(crate) fn new(info: AttributeInfo<'s>, node: &'s HoudiniNode) -> Self {
        Attribute::<T> {
            info,
            node,
            _marker: Default::default(),
        }
    }
    pub fn read(&self, part_id: i32) -> Result<T::Return> {
        T::read(self.node, part_id, &self.info)
    }
}

macro_rules! impl_attrib_type {
    ($ty:ty, $func:ident) => {
        impl AttribDataType for $ty {
            type Type = $ty;
            type Return = Vec<Self::Type>;
            fn read<'session>(
                node: &HoudiniNode,
                part_id: i32,
                info: &AttributeInfo<'session>,
            ) -> Result<Vec<Self>> {
                crate::ffi::$func(node, part_id, &info.name, &info.inner, -1, 0, info.count())
            }

            fn set(values: impl AsRef<[Self]>) -> Result<()> {
                unimplemented!()
            }
        }
    };
}

impl_attrib_type!(f32, get_attribute_float_data);
impl_attrib_type!(f64, get_attribute_float64_data);
impl_attrib_type!(i32, get_attribute_int_data);
impl_attrib_type!(i64, get_attribute_int64_data);

impl<'a> AttribDataType for &'a str {
    type Type = &'a str;
    type Return = StringsArray;

    fn read<'s>(
        node: &'s HoudiniNode,
        part_id: i32,
        info: &'s AttributeInfo<'_>,
    ) -> Result<Self::Return> {
        crate::ffi::get_attribute_string_buffer(
            node,
            part_id,
            &info.name,
            &info.inner,
            0,
            info.count(),
        )
    }

    fn set(values: impl AsRef<[Self::Type]>) -> Result<()> {
        unimplemented!()
    }
}
