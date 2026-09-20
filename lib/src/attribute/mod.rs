//! Typed geometry attribute handles.

mod array;
#[cfg(feature = "async-cooking")]
mod async_;

pub use crate::ffi::AttributeInfo;
pub use crate::ffi::enums::StorageType;
pub use array::{JaggedArrayData, JaggedArrayIter, StringJaggedArrayData, StringJaggedArrayIter};
#[cfg(feature = "async-cooking")]
pub use async_::{
    AsyncAttributeAccess, AsyncFixedAttributeAccess, AsyncJob, AsyncStringAttributeAccess,
};

use crate::errors::{HapiError, Result};
#[doc(hidden)]
pub use crate::ffi::NumericPrimitive;
use crate::ffi::enums::AttributeOwner;
use crate::node::HoudiniNode;
use crate::stringhandle::StringArray;
use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub struct Fixed;
#[derive(Debug, Clone, Copy)]
pub struct Jagged;

mod private {
    pub trait Sealed {}
}
impl private::Sealed for Fixed {}
impl private::Sealed for Jagged {}

pub trait AttributeShape: private::Sealed + Send + 'static {
    fn storage<T: NumericPrimitive>() -> StorageType;
}
impl AttributeShape for Fixed {
    fn storage<T: NumericPrimitive>() -> StorageType {
        T::FIXED_STORAGE
    }
}
impl AttributeShape for Jagged {
    fn storage<T: NumericPrimitive>() -> StorageType {
        T::JAGGED_STORAGE
    }
}

#[derive(Debug, Clone)]
struct Handle {
    info: AttributeInfo,
    name: CString,
    node: HoudiniNode,
    part_id: i32,
    owner: AttributeOwner,
    newly_created: bool,
}

impl Handle {
    fn new(
        name: CString,
        info: AttributeInfo,
        node: HoudiniNode,
        part_id: i32,
        newly_created: bool,
    ) -> Self {
        let owner = info.owner();
        Self {
            info,
            name,
            node,
            part_id,
            owner,
            newly_created,
        }
    }

    fn refresh(&self, expected: StorageType) -> Result<AttributeInfo> {
        let info = AttributeInfo::new(&self.node, self.part_id, self.owner, &self.name)?;
        if !info.exists() {
            if self.newly_created {
                return Ok(self.info.clone());
            }
            return Err(HapiError::Internal(format!(
                "attribute {:?} no longer exists on part {} for owner {:?}",
                self.name, self.part_id, self.owner
            )));
        }
        if info.storage() != expected {
            return Err(HapiError::Internal(format!(
                "attribute {:?} storage changed from {:?} to {:?}",
                self.name,
                expected,
                info.storage()
            )));
        }
        Ok(info)
    }

    fn expected_fixed_len(info: &AttributeInfo) -> Result<usize> {
        let len = info
            .count()
            .checked_mul(info.tuple_size())
            .ok_or_else(|| HapiError::Internal("attribute buffer length overflow".into()))?;
        usize::try_from(len)
            .map_err(|_| HapiError::Internal("attribute buffer length is negative".into()))
    }
}

#[derive(Debug, Clone)]
pub struct Attribute<T: NumericPrimitive, S: AttributeShape> {
    handle: Handle,
    marker: PhantomData<(T, S)>,
}

impl<T: NumericPrimitive, S: AttributeShape> Attribute<T, S> {
    pub(crate) fn new(name: CString, info: AttributeInfo, node: HoudiniNode, part_id: i32) -> Self {
        Self {
            handle: Handle::new(name, info, node, part_id, false),
            marker: PhantomData,
        }
    }
    pub(crate) fn new_created(
        name: CString,
        info: AttributeInfo,
        node: HoudiniNode,
        part_id: i32,
    ) -> Self {
        Self {
            handle: Handle::new(name, info, node, part_id, true),
            marker: PhantomData,
        }
    }
    #[must_use]
    pub fn info(&self) -> &AttributeInfo {
        &self.handle.info
    }
    #[must_use]
    pub fn name(&self) -> &CStr {
        &self.handle.name
    }
    #[must_use]
    pub fn part_id(&self) -> i32 {
        self.handle.part_id
    }
    #[must_use]
    pub fn owner(&self) -> AttributeOwner {
        self.handle.owner
    }
    #[must_use]
    pub fn storage(&self) -> StorageType {
        S::storage::<T>()
    }
    pub fn delete(self) -> Result<()> {
        crate::ffi::delete_attribute(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &self.handle.info.0,
        )
    }
}

impl<T: NumericPrimitive> Attribute<T, Fixed> {
    pub fn get(&self) -> Result<Vec<T>> {
        let info = self.handle.refresh(T::FIXED_STORAGE)?;
        let mut data = vec![T::default(); Handle::expected_fixed_len(&info)?];
        T::get_fixed(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            &mut data,
        )?;
        Ok(data)
    }

    pub fn read_into(&self, data: &mut Vec<T>) -> Result<()> {
        let info = self.handle.refresh(T::FIXED_STORAGE)?;
        data.resize(Handle::expected_fixed_len(&info)?, T::default());
        T::get_fixed(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            data,
        )
    }

    pub fn set(&self, data: &[T]) -> Result<()> {
        let info = self.handle.refresh(T::FIXED_STORAGE)?;
        let expected = Handle::expected_fixed_len(&info)?;
        if data.len() != expected {
            return Err(HapiError::Internal(format!(
                "attribute {:?} needs {expected} values, got {}",
                self.handle.name,
                data.len()
            )));
        }
        T::set_fixed(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            data,
            0,
            info.count(),
        )
    }

    pub fn set_unique(&self, value: &[T]) -> Result<()> {
        let info = self.handle.refresh(T::FIXED_STORAGE)?;
        let expected = usize::try_from(info.tuple_size())
            .map_err(|_| HapiError::Internal("negative tuple size".into()))?;
        if value.len() != expected {
            return Err(HapiError::Internal(format!(
                "unique value needs {expected} tuple values, got {}",
                value.len()
            )));
        }
        T::set_unique(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            value,
        )
    }
}

impl<T: NumericPrimitive> Attribute<T, Jagged> {
    pub fn get(&self) -> Result<JaggedArrayData<T>> {
        let info = self.handle.refresh(T::JAGGED_STORAGE)?;
        let total = usize::try_from(info.total_array_elements())
            .map_err(|_| HapiError::Internal("negative jagged element count".into()))?;
        let count = usize::try_from(info.count())
            .map_err(|_| HapiError::Internal("negative attribute count".into()))?;
        let mut data = vec![T::default(); total];
        let mut sizes = vec![0; count];
        T::get_jagged(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            &mut data,
            &mut sizes,
        )?;
        JaggedArrayData::from_hapi(data, sizes)
    }

    pub fn set(&self, values: &JaggedArrayData<T>) -> Result<()> {
        let info = self.handle.refresh(T::JAGGED_STORAGE)?;
        let expected = usize::try_from(info.count())
            .map_err(|_| HapiError::Internal("negative attribute count".into()))?;
        if values.sizes().len() != expected {
            return Err(HapiError::Internal(format!(
                "attribute {:?} needs {expected} array sizes, got {}",
                self.handle.name,
                values.sizes().len()
            )));
        }
        T::set_jagged(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            values.data(),
            values.sizes(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct StringAttribute<S: AttributeShape> {
    handle: Handle,
    marker: PhantomData<S>,
}
#[derive(Debug, Clone)]
pub struct DictionaryAttribute<S: AttributeShape> {
    handle: Handle,
    marker: PhantomData<S>,
}

macro_rules! common_handle {
    ($type:ident) => {
        impl<S: AttributeShape> $type<S> {
            pub(crate) fn new(
                name: CString,
                info: AttributeInfo,
                node: HoudiniNode,
                part_id: i32,
            ) -> Self {
                Self {
                    handle: Handle::new(name, info, node, part_id, false),
                    marker: PhantomData,
                }
            }
            pub(crate) fn new_created(
                name: CString,
                info: AttributeInfo,
                node: HoudiniNode,
                part_id: i32,
            ) -> Self {
                Self {
                    handle: Handle::new(name, info, node, part_id, true),
                    marker: PhantomData,
                }
            }
            #[must_use]
            pub fn info(&self) -> &AttributeInfo {
                &self.handle.info
            }
            #[must_use]
            pub fn name(&self) -> &CStr {
                &self.handle.name
            }
            #[must_use]
            pub fn part_id(&self) -> i32 {
                self.handle.part_id
            }
            #[must_use]
            pub fn owner(&self) -> AttributeOwner {
                self.handle.owner
            }
            #[must_use]
            pub fn storage(&self) -> StorageType {
                self.handle.info.storage()
            }
            pub fn delete(self) -> Result<()> {
                crate::ffi::delete_attribute(
                    &self.handle.node,
                    self.handle.part_id,
                    &self.handle.name,
                    &self.handle.info.0,
                )
            }
        }
    };
}
common_handle!(StringAttribute);
common_handle!(DictionaryAttribute);

macro_rules! fixed_string_impl {
    ($type:ident, $storage:expr, $dictionary:expr) => {
        impl $type<Fixed> {
            pub fn get(&self) -> Result<StringArray> {
                let info = self.handle.refresh($storage)?;
                crate::ffi::get_string_attribute_data(
                    &self.handle.node,
                    self.handle.part_id,
                    &self.handle.name,
                    &info.0,
                    $dictionary,
                )
            }
            pub fn set(&self, values: &[impl AsRef<CStr>]) -> Result<()> {
                let info = self.handle.refresh($storage)?;
                let expected = Handle::expected_fixed_len(&info)?;
                if values.len() != expected {
                    return Err(HapiError::Internal(format!(
                        "attribute {:?} needs {expected} strings, got {}",
                        self.handle.name,
                        values.len()
                    )));
                }
                let ptrs: Vec<_> = values.iter().map(|v| v.as_ref().as_ptr()).collect();
                crate::ffi::set_string_attribute_data(
                    &self.handle.node,
                    self.handle.part_id,
                    &self.handle.name,
                    &info.0,
                    &ptrs,
                    $dictionary,
                )
            }
        }
    };
}
fixed_string_impl!(StringAttribute, StorageType::String, false);
fixed_string_impl!(DictionaryAttribute, StorageType::Dictionary, true);

impl StringAttribute<Fixed> {
    pub fn set_unique(&self, value: &CStr) -> Result<()> {
        let info = self.handle.refresh(StorageType::String)?;
        crate::ffi::set_string_unique_attribute_data(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            value,
        )
    }
    pub fn set_indexed(&self, values: &[impl AsRef<CStr>], indices: &[i32]) -> Result<()> {
        let info = self.handle.refresh(StorageType::String)?;
        let expected = Handle::expected_fixed_len(&info)?;
        if indices.len() != expected {
            return Err(HapiError::Internal(format!(
                "attribute {:?} needs {expected} indices, got {}",
                self.handle.name,
                indices.len()
            )));
        }
        let ptrs: Vec<_> = values.iter().map(|v| v.as_ref().as_ptr()).collect();
        crate::ffi::set_indexed_string_attribute_data(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            &ptrs,
            indices,
        )
    }
}

macro_rules! jagged_string_impl {
    ($type:ident, $storage:expr, $dictionary:expr) => {
        impl $type<Jagged> {
            pub fn get(&self) -> Result<StringJaggedArrayData> {
                let info = self.handle.refresh($storage)?;
                let (handles, sizes) = crate::ffi::get_string_jagged_attribute_data(
                    &self.handle.node,
                    self.handle.part_id,
                    &self.handle.name,
                    &info.0,
                    $dictionary,
                )?;
                StringJaggedArrayData::from_hapi(handles, sizes, self.handle.node.session.clone())
            }
            pub fn set(&self, values: &[impl AsRef<CStr>], sizes: &[i32]) -> Result<()> {
                let info = self.handle.refresh($storage)?;
                let validated =
                    JaggedArrayData::new(values.iter().map(|_| ()).collect(), sizes.to_vec())?;
                let expected = usize::try_from(info.count())
                    .map_err(|_| HapiError::Internal("negative attribute count".into()))?;
                if validated.sizes().len() != expected {
                    return Err(HapiError::Internal(format!(
                        "attribute {:?} needs {expected} array sizes, got {}",
                        self.handle.name,
                        sizes.len()
                    )));
                }
                let ptrs: Vec<_> = values.iter().map(|v| v.as_ref().as_ptr()).collect();
                crate::ffi::set_string_jagged_attribute_data(
                    &self.handle.node,
                    self.handle.part_id,
                    &self.handle.name,
                    &info.0,
                    &ptrs,
                    sizes,
                    $dictionary,
                )
            }
        }
    };
}
jagged_string_impl!(StringAttribute, StorageType::StringArray, false);
jagged_string_impl!(DictionaryAttribute, StorageType::DictionaryArray, true);

#[derive(Debug, Clone)]
pub enum AnyAttribute {
    Int(Attribute<i32, Fixed>),
    Int64(Attribute<i64, Fixed>),
    Float(Attribute<f32, Fixed>),
    Float64(Attribute<f64, Fixed>),
    String(StringAttribute<Fixed>),
    Uint8(Attribute<u8, Fixed>),
    Int8(Attribute<i8, Fixed>),
    Int16(Attribute<i16, Fixed>),
    IntArray(Attribute<i32, Jagged>),
    Int64Array(Attribute<i64, Jagged>),
    FloatArray(Attribute<f32, Jagged>),
    Float64Array(Attribute<f64, Jagged>),
    StringArray(StringAttribute<Jagged>),
    Uint8Array(Attribute<u8, Jagged>),
    Int8Array(Attribute<i8, Jagged>),
    Int16Array(Attribute<i16, Jagged>),
    Dictionary(DictionaryAttribute<Fixed>),
    DictionaryArray(DictionaryAttribute<Jagged>),
}

impl AnyAttribute {
    fn handle(&self) -> &Handle {
        match self {
            Self::Int(v) => &v.handle,
            Self::Int64(v) => &v.handle,
            Self::Float(v) => &v.handle,
            Self::Float64(v) => &v.handle,
            Self::String(v) => &v.handle,
            Self::Uint8(v) => &v.handle,
            Self::Int8(v) => &v.handle,
            Self::Int16(v) => &v.handle,
            Self::IntArray(v) => &v.handle,
            Self::Int64Array(v) => &v.handle,
            Self::FloatArray(v) => &v.handle,
            Self::Float64Array(v) => &v.handle,
            Self::StringArray(v) => &v.handle,
            Self::Uint8Array(v) => &v.handle,
            Self::Int8Array(v) => &v.handle,
            Self::Int16Array(v) => &v.handle,
            Self::Dictionary(v) => &v.handle,
            Self::DictionaryArray(v) => &v.handle,
        }
    }
    #[must_use]
    pub fn name(&self) -> Cow<'_, str> {
        self.handle().name.to_string_lossy()
    }
    #[must_use]
    pub fn info(&self) -> &AttributeInfo {
        &self.handle().info
    }
    #[must_use]
    pub fn storage(&self) -> StorageType {
        self.handle().info.storage()
    }
    #[must_use]
    pub fn part_id(&self) -> i32 {
        self.handle().part_id
    }
    #[must_use]
    pub fn owner(&self) -> AttributeOwner {
        self.handle().owner
    }
    pub fn delete(self) -> Result<()> {
        let h = self.handle();
        crate::ffi::delete_attribute(&h.node, h.part_id, &h.name, &h.info.0)
    }
}
