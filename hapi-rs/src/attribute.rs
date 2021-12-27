use crate::errors::Result;
pub use crate::ffi::enums::{StorageType};
pub use crate::ffi::AttributeInfo;
use crate::node::HoudiniNode;
use crate::stringhandle::StringArray;
use duplicate::duplicate;
use std::borrow::Cow;
use std::ffi::{CStr, CString};

pub struct DataArray<T> {
    pub data: Vec<T>,
    pub sizes: Vec<i32>,
}

pub struct StringMultiArray {
    pub handles: Vec<i32>,
    pub sizes: Vec<i32>,
    session: crate::session::Session,
}

impl<T> DataArray<T> {
    pub fn iter(&self) -> ArrayIter<'_, T> {
        ArrayIter {
            data: self.data.iter(),
            sizes: self.sizes.iter(),
            cursor: 0,
        }
    }
}

pub struct ArrayIter<'a, T> {
    data: std::slice::Iter<'a, T>,
    sizes: std::slice::Iter<'a, i32>,
    cursor: usize,
}

pub struct MultiArrayIter<'a> {
    handles: std::slice::Iter<'a, i32>,
    sizes: std::slice::Iter<'a, i32>,
    session: &'a crate::session::Session,
    cursor: usize,
}

impl<'a, T> Iterator for ArrayIter<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        match self.sizes.next() {
            None => None,
            Some(size) => {
                let start = self.cursor;
                let end = self.cursor + (*size as usize);
                self.cursor = end;
                // TODO: We know the data size, it can be rewritten to use unsafe unchecked
                Some(&self.data.as_slice()[start..end])
            }
        }
    }
}

impl StringMultiArray {
    pub fn iter(&self) -> MultiArrayIter<'_> {
        MultiArrayIter {
            handles: self.handles.iter(),
            sizes: self.sizes.iter(),
            session: &self.session,
            cursor: 0,
        }
    }
}

impl<'a> Iterator for MultiArrayIter<'a> {
    type Item = Result<StringArray>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.sizes.next() {
            None => None,
            Some(size) => {
                let start = self.cursor;
                let end = self.cursor + (*size as usize);
                self.cursor = end;
                let handles = &self.handles.as_slice()[start..end];
                Some(crate::stringhandle::get_string_array(handles, self.session))
            }
        }
    }
}

pub trait AttribDataType: Sized {
    type Type;
    type Return;
    type ArrayType;
    fn read(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::Return>;
    fn read_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::ArrayType>;
    fn set(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        values: &[Self::Type],
    ) -> Result<()>;
    fn storage() -> StorageType;
}

#[derive(Debug)]
pub struct Attribute<'s, T: AttribDataType> {
    pub info: AttributeInfo,
    pub(crate) name: CString,
    pub(crate) node: &'s HoudiniNode,
    _marker: std::marker::PhantomData<T>,
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

    pub fn name(&self) -> Cow<str> {
        self.name.to_string_lossy()
    }
    pub fn read(&self, part_id: i32) -> Result<T::Return> {
        T::read(&self.name, self.node, part_id, &self.info)
    }

    pub fn read_array(&self, part_id: i32) -> Result<T::ArrayType> {
        T::read_array(&self.name, self.node, part_id, &self.info)
    }

    pub fn set(&self, part_id: i32, values: impl AsRef<[T::Type]>) -> Result<()> {
        T::set(&self.name, self.node, part_id, &self.info, values.as_ref())
    }
}

#[duplicate(
    _data_type  _storage           _get                      _set                      _array;
    [u8]    [StorageType::Uint8] [get_attribute_u8_data] [set_attribute_u8_data] [get_attribute_u8_array_data];
    [i8]    [StorageType::Int8] [get_attribute_i8_data] [set_attribute_i8_data] [get_attribute_i8_array_data];
    [i16]    [StorageType::Int16] [get_attribute_i16_data] [set_attribute_i16_data] [get_attribute_i16_array_data];
    [i32]    [StorageType::Int] [get_attribute_int_data] [set_attribute_int_data] [get_attribute_int_array_data];
    [i64]    [StorageType::Int64] [get_attribute_int64_data] [set_attribute_int64_data] [get_attribute_int64_array_data];
    [f32]    [StorageType::Float] [get_attribute_float_data] [set_attribute_float_data] [get_attribute_float_array_data];
    [f64]    [StorageType::Float64] [get_attribute_float64_data] [set_attribute_float64_data] [get_attribute_float64_array_data];
)]
impl AttribDataType for _data_type {
    type Type = _data_type;
    type Return = Vec<Self::Type>;
    type ArrayType = DataArray<_data_type>;

    fn read(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::Return> {
        crate::ffi::_get(node, part_id, name, &info.inner, -1, 0, info.count())
    }

    fn set(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        values: &[Self::Type],
    ) -> Result<()> {
        crate::ffi::_set(node, part_id, name, &info.inner, values, 0, info.count())
    }

    fn read_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Self::ArrayType> {
        crate::ffi::_array(node, part_id, name, &info.inner)
    }

    fn storage() -> StorageType {
        _storage
    }
}

impl<'a> AttribDataType for &'a str {
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

    fn storage() -> StorageType {
        StorageType::String
    }
}
