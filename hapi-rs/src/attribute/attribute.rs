use super::array::{DataArray, StringMultiArray};
use crate::errors::Result;
pub use crate::ffi::enums::StorageType;
pub use crate::ffi::AttributeInfo;
use crate::node::HoudiniNode;
use crate::stringhandle::StringArray;
use duplicate::duplicate;
use std::borrow::Cow;
use std::ffi::{CStr, CString};

pub trait AttribType: AttributeAccess {
    fn storage() -> StorageType;
}

macro_rules! impl_foo {
    ($tp:ty, $st:expr) => {
        impl AttribType for $tp {
            fn storage() -> StorageType {
                $st
            }
        }
    };
}

impl_foo!(i8, StorageType::Int8);
impl_foo!(u8, StorageType::Uint8);
impl_foo!(i16, StorageType::Int16);
impl_foo!(i32, StorageType::Int);
impl_foo!(i64, StorageType::Int64);
impl_foo!(f32, StorageType::Float);
impl_foo!(f64, StorageType::Float64);
impl_foo!(&[i8], StorageType::Int8Array);
// impl_foo!(&[u8], StorageType::Uint8Array);
// impl_foo!(&[i16], StorageType::Int16Array);
// impl_foo!(&[i32], StorageType::Array);
// impl_foo!(&[f32], StorageType::FloatArray);
// impl_foo!(&[f64], StorageType::Float64Array);
impl_foo!(&str, StorageType::String);

pub trait AttributeAccess: Sized {
    type Type;
    type Return;
    type ArrayType;
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
    fn read_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::ArrayType>;
    fn set_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        data: &Self::ArrayType,
    ) -> Result<()>;
}

#[derive(Debug)]
pub struct Attribute<'s, T: AttribType> {
    pub info: AttributeInfo,
    pub(crate) name: CString,
    pub(crate) node: &'s HoudiniNode,
    _marker: std::marker::PhantomData<T>,
}

impl<'s, T> Attribute<'s, T>
where
    T: AttribType,
{
    pub(crate) fn new(name: CString, info: AttributeInfo, node: &'s HoudiniNode) -> Self {
        Attribute::<T> {
            info,
            node,
            name,
            _marker: Default::default(),
        }
    }

    pub fn name(&self) -> Cow<str> {
        self.name.to_string_lossy()
    }
    pub fn read(&self, part_id: i32) -> Result<T::Return> {
        T::read(&self.name, self.node, part_id, &self.info)
    }

    pub fn read_array(&self, part_id: i32) -> Result<T::ArrayType> {
        T::read_array(&self.name, self.node, part_id, &self.info)
    }
    pub fn set_array(&self, part_id: i32, data: &T::ArrayType) -> Result<()> {
        T::set_array(&self.name, self.node, part_id, &self.info, data)
    }

    pub fn set(&self, part_id: i32, values: impl AsRef<[T::Type]>) -> Result<()> {
        T::set(&self.name, self.node, part_id, &self.info, values.as_ref())
    }
}

#[duplicate(
    _data_type  _get                      _set                      _get_array           _set_array;
    [u8]    [get_attribute_u8_data] [set_attribute_u8_data] [get_attribute_u8_array_data] [set_attribute_u8_array_data];
    [i8]    [get_attribute_i8_data] [set_attribute_i8_data] [get_attribute_i8_array_data] [set_attribute_i8_array_data];
    [i16]    [get_attribute_i16_data] [set_attribute_i16_data] [get_attribute_i16_array_data] [set_attribute_i16_array_data];
    [i32]    [get_attribute_int_data] [set_attribute_int_data] [get_attribute_int_array_data] [set_attribute_i32_array_data];
    [i64]    [get_attribute_int64_data] [set_attribute_int64_data] [get_attribute_int64_array_data] [set_attribute_i64_array_data];
    [f32]    [get_attribute_float_data] [set_attribute_float_data] [get_attribute_float_array_data] [set_attribute_f32_array_data];
    [f64]    [get_attribute_float64_data] [set_attribute_float64_data] [get_attribute_float64_array_data] [set_attribute_f64_array_data];
)]
impl AttributeAccess for _data_type {
    type Type = _data_type;
    type Return = Vec<Self::Type>;
    type ArrayType = DataArray<_data_type>;

    fn read(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::Return> {
        super::bindings::_get(node, part_id, name, &info.inner, -1, 0, info.count())
    }

    fn set(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        values: &[Self::Type],
    ) -> Result<()> {
        super::bindings::_set(node, part_id, name, &info.inner, values, 0, info.count())
    }

    fn read_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::ArrayType> {
        super::bindings::_get_array(node, part_id, name, &info.inner)
    }

    fn set_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        data: &Self::ArrayType,
    ) -> Result<()> {
        super::bindings::_set_array(node, part_id, name, &info.inner, &data.data, &data.sizes)
    }
}

impl<'a> AttributeAccess for &'a str {
    type Type = &'a str;
    type Return = StringArray;
    type ArrayType = StringMultiArray;

    fn read(
        name: &CStr,
        node: &HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::Return> {
        crate::ffi::get_attribute_string_buffer(node, part_id, name, &info.inner, 0, info.count())
    }

    fn set(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        values: &[Self::Type],
    ) -> Result<()> {
        let cstrings = values
            .iter()
            .map(|s| CString::new(*s).map_err(Into::into))
            .collect::<Result<Vec<CString>>>()?;
        let cstrings = cstrings.iter().map(CString::as_ref).collect::<Vec<_>>();
        crate::ffi::set_attribute_string_buffer(
            &node.session,
            node.handle,
            part_id,
            name,
            &info.inner,
            &cstrings,
        )
    }
    fn read_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        _part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::ArrayType> {
        let (handles, sizes) = crate::ffi::get_attribute_string_array_data(
            &node.session,
            node.handle,
            name,
            &info.inner,
        )?;
        Ok(StringMultiArray {
            handles,
            sizes,
            session: node.session.clone(),
        })
    }

    fn set_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        data: &Self::ArrayType,
    ) -> Result<()> {
        todo!()
    }
}
