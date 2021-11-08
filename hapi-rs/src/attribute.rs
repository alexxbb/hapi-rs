use std::marker::PhantomData;

use crate::errors::Result;
pub use crate::ffi::raw::StorageType;
pub use crate::ffi::AttributeInfo;
use crate::node::HoudiniNode;
use crate::stringhandle::StringsArray;
use std::ffi::{CStr, CString};

pub trait AttribDataType: Sized {
    type Type;
    type Return;
    fn read(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::Return>;
    fn set(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        values: &[Self::Type],
    ) -> Result<()>;
}

#[derive(Debug)]
pub struct Attribute<'s, T: AttribDataType> {
    pub info: AttributeInfo,
    pub(crate) node: &'s HoudiniNode,
    pub(crate) name: CString,
    _marker: PhantomData<T>,
}

impl<'s, T> Attribute<'s, T>
where
    T: AttribDataType,
{
    pub(crate) fn new(name: CString, info: AttributeInfo, node: &'s HoudiniNode) -> Self {
        Attribute::<T> {
            info,
            node,
            name,
            _marker: Default::default(),
        }
    }
    pub fn read(&self, part_id: i32) -> Result<T::Return> {
        T::read(&self.name, self.node, part_id, &self.info)
    }

    pub fn set(&self, part_id: i32, values: impl AsRef<[T::Type]>) -> Result<()> {
        T::set(&self.name, &self.node, part_id, &self.info, values.as_ref())
    }
}

macro_rules! impl_attrib_type {
    ($ty:ty, $get_func:ident, $set_func:ident) => {
        impl AttribDataType for $ty {
            type Type = $ty;
            type Return = Vec<Self::Type>;
            fn read<'session>(
                name: &CStr,
                node: &HoudiniNode,
                part_id: i32,
                info: &AttributeInfo,
            ) -> Result<Vec<Self>> {
                crate::ffi::$get_func(node, part_id, name, &info.inner, -1, 0, info.count())
            }

            fn set(
                name: &CStr,
                node: &HoudiniNode,
                part_id: i32,
                info: &AttributeInfo,
                values: &[Self::Type],
            ) -> Result<()> {
                crate::ffi::$set_func(node, part_id, name, &info.inner, values, 0, info.count())
            }
        }
    };
}

impl_attrib_type!(f32, get_attribute_float_data, set_attribute_float_data);
impl_attrib_type!(f64, get_attribute_float64_data, set_attribute_float64_data);
impl_attrib_type!(i32, get_attribute_int_data, set_attribute_int_data);
impl_attrib_type!(i64, get_attribute_int64_data, set_attribute_int64_data);

impl<'a> AttribDataType for &'a str {
    type Type = &'a str;
    type Return = StringsArray;

    fn read(
        name: &CStr,
        node: &HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::Return> {
        crate::ffi::get_attribute_string_buffer(node, part_id, name, &info.inner, 0, info.count())
    }

    #[allow(unused_variables)]
    fn set(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        values: &[Self::Type],
    ) -> Result<()> {
        todo!()
    }
}
