//! Typed handles for geometry attributes.
//!
//! Attribute handles are bound to a node, part, owner, and name when they are
//! created or looked up. Consequently, [`Attribute::get`] and
//! [`Attribute::set`] do not accept a part id: use a separate lookup to obtain
//! a handle for another part. HAPI part ids are indices that can change after a
//! cook, so reacquire handles after a cook that may change the part layout.
//!
//! The shape marker distinguishes ordinary fixed-tuple attributes from HAPI
//! array attributes:
//!
//! - [`Attribute<T, Fixed>`](Attribute) reads and writes a flat `Vec<T>` whose
//!   length is `count * tuple_size`.
//! - [`Attribute<T, Jagged>`](Attribute) uses [`JaggedArrayData<T>`], containing
//!   flattened values plus one array size for every owned element.
//! - [`StringAttribute`] and [`DictionaryAttribute`] follow the same
//!   [`Fixed`]/[`Jagged`] distinction.
//!
//! Fixed attributes also expose checked range operations. Range indices refer
//! to owned geometry elements, not flattened tuple values.

mod array;
#[cfg(feature = "async-cooking")]
mod async_;

pub use crate::ffi::AttributeInfo;
pub use crate::ffi::enums::StorageType;
pub use array::{JaggedArrayData, JaggedArrayIter, StringJaggedArrayData, StringJaggedArrayIter};
#[cfg(feature = "async-cooking")]
pub use async_::{
    AsyncAttributeAccess, AsyncFixedAttributeAccess, AsyncJob, AsyncStringAttributeAccess,
    AsyncStringAttributeWrite,
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
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
/// Marker for an ordinary HAPI attribute with a fixed tuple size.
pub struct Fixed;
#[derive(Debug, Clone, Copy)]
/// Marker for a HAPI array attribute whose entries may have different lengths.
pub struct Jagged;

mod private {
    pub trait Sealed {}
}
impl private::Sealed for Fixed {}
impl private::Sealed for Jagged {}

/// Sealed mapping from an attribute shape and numeric Rust type to HAPI storage.
///
/// Implemented by [`Fixed`] and [`Jagged`]. It is primarily used as a generic
/// bound on typed lookup and creation methods.
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
}

impl Handle {
    fn new(name: CString, info: AttributeInfo, node: HoudiniNode, part_id: i32) -> Self {
        let owner = info.owner();
        Self {
            info,
            name,
            node,
            part_id,
            owner,
        }
    }

    fn refresh(&mut self, expected: StorageType) -> Result<()> {
        let info = AttributeInfo::new(&self.node, self.part_id, self.owner, &self.name)?;
        if !info.exists() {
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
        self.info = info;
        Ok(())
    }

    fn expected_fixed_len(info: &AttributeInfo) -> Result<usize> {
        let len = info
            .count()
            .checked_mul(info.tuple_size())
            .ok_or_else(|| HapiError::Internal("attribute buffer length overflow".into()))?;
        usize::try_from(len)
            .map_err(|_| HapiError::Internal("attribute buffer length is negative".into()))
    }

    fn checked_fixed_range(&self, range: Range<usize>) -> Result<(i32, i32, usize)> {
        if range.start > range.end {
            return Err(HapiError::Internal(format!(
                "attribute {:?} range start {} exceeds end {}",
                self.name, range.start, range.end
            )));
        }
        let count = usize::try_from(self.info.count())
            .map_err(|_| HapiError::Internal("negative attribute count".into()))?;
        if range.end > count {
            return Err(HapiError::Internal(format!(
                "attribute {:?} range end {} exceeds count {count}",
                self.name, range.end
            )));
        }
        let tuple_size = usize::try_from(self.info.tuple_size())
            .map_err(|_| HapiError::Internal("negative attribute tuple size".into()))?;
        let value_len = (range.end - range.start)
            .checked_mul(tuple_size)
            .ok_or_else(|| HapiError::Internal("attribute range length overflow".into()))?;
        let start = i32::try_from(range.start)
            .map_err(|_| HapiError::Internal("attribute range start exceeds i32".into()))?;
        let count = i32::try_from(range.end - range.start)
            .map_err(|_| HapiError::Internal("attribute range length exceeds i32".into()))?;
        Ok((start, count, value_len))
    }
}

fn validate_string_indices(value_count: usize, indices: &[i32]) -> Result<()> {
    for (position, &index) in indices.iter().enumerate() {
        let index = usize::try_from(index).map_err(|_| {
            HapiError::Internal(format!(
                "indexed string index at position {position} is negative: {index}"
            ))
        })?;
        if index >= value_count {
            return Err(HapiError::Internal(format!(
                "indexed string index at position {position} is {index}, but the value table has {value_count} entries"
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
/// A typed numeric geometry attribute handle.
///
/// `T` determines the numeric primitive storage and `S` determines whether the
/// attribute is fixed-tuple or jagged. The handle retains its node, part,
/// owner, and name identity. Data operations use cached metadata so they do not
/// add a hidden HAPI query to hot paths.
pub struct Attribute<T: NumericPrimitive, S: AttributeShape> {
    handle: Handle,
    marker: PhantomData<(T, S)>,
}

impl<T: NumericPrimitive, S: AttributeShape> Attribute<T, S> {
    pub(crate) fn new(name: CString, info: AttributeInfo, node: HoudiniNode, part_id: i32) -> Self {
        Self {
            handle: Handle::new(name, info, node, part_id),
            marker: PhantomData,
        }
    }
    /// Returns the currently cached metadata.
    #[must_use]
    pub fn info(&self) -> &AttributeInfo {
        &self.handle.info
    }
    /// Returns the attribute name.
    #[must_use]
    pub fn name(&self) -> &CStr {
        &self.handle.name
    }
    /// Returns the HAPI part to which this handle is bound.
    #[must_use]
    pub fn part_id(&self) -> i32 {
        self.handle.part_id
    }
    /// Returns the element owner of this attribute.
    #[must_use]
    pub fn owner(&self) -> AttributeOwner {
        self.handle.owner
    }
    /// Returns the HAPI storage implied by `T` and `S`.
    #[must_use]
    pub fn storage(&self) -> StorageType {
        S::storage::<T>()
    }
    /// Refreshes cached metadata from HAPI.
    ///
    /// Call this after cooking or otherwise changing geometry when the
    /// attribute's count, tuple size, or array element total may have changed.
    /// The cache is updated only if the attribute still exists and retains the
    /// numeric storage implied by `T` and `S`.
    pub fn refresh(&mut self) -> Result<()> {
        self.handle.refresh(S::storage::<T>())
    }
    /// Deletes this attribute from its bound node and part.
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
    /// Reads the complete fixed-tuple attribute as a flat vector.
    ///
    /// The result length is `count * tuple_size` using cached metadata.
    pub fn get(&self) -> Result<Vec<T>> {
        let info = &self.handle.info;
        let mut data = vec![T::default(); Handle::expected_fixed_len(info)?];
        T::get_fixed(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            &mut data,
            0,
            info.count(),
        )?;
        Ok(data)
    }

    /// Reads the complete attribute into a reusable vector.
    ///
    /// `data` is resized to `count * tuple_size` before the HAPI call.
    pub fn read_into(&self, data: &mut Vec<T>) -> Result<()> {
        let info = &self.handle.info;
        data.resize(Handle::expected_fixed_len(info)?, T::default());
        T::get_fixed(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            data,
            0,
            info.count(),
        )
    }

    /// Reads a range of owned elements as flattened tuples.
    pub fn get_range(&self, range: Range<usize>) -> Result<Vec<T>> {
        let (start, count, value_len) = self.handle.checked_fixed_range(range)?;
        let mut data = vec![T::default(); value_len];
        if count == 0 {
            return Ok(data);
        }
        T::get_fixed(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &self.handle.info.0,
            &mut data,
            start,
            count,
        )?;
        Ok(data)
    }

    /// Reads a range of owned elements into a reusable flattened vector.
    pub fn read_range_into(&self, range: Range<usize>, data: &mut Vec<T>) -> Result<()> {
        let (start, count, value_len) = self.handle.checked_fixed_range(range)?;
        data.resize(value_len, T::default());
        if count == 0 {
            return Ok(());
        }
        T::get_fixed(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &self.handle.info.0,
            data,
            start,
            count,
        )
    }

    /// Replaces the complete fixed-tuple attribute.
    ///
    /// Returns an error unless `data.len() == count * tuple_size`.
    pub fn set(&self, data: &[T]) -> Result<()> {
        let info = &self.handle.info;
        let expected = Handle::expected_fixed_len(info)?;
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

    /// Replaces a range of owned elements from flattened tuples.
    pub fn set_range(&self, range: Range<usize>, data: &[T]) -> Result<()> {
        let (start, count, expected) = self.handle.checked_fixed_range(range)?;
        if data.len() != expected {
            return Err(HapiError::Internal(format!(
                "attribute {:?} range needs {expected} values, got {}",
                self.handle.name,
                data.len()
            )));
        }
        if count == 0 {
            return Ok(());
        }
        T::set_fixed(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &self.handle.info.0,
            data,
            start,
            count,
        )
    }

    /// Assigns the same tuple value to every element of the attribute.
    ///
    /// `value` must contain exactly `tuple_size` numeric values.
    pub fn set_unique(&self, value: &[T]) -> Result<()> {
        let info = &self.handle.info;
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
    /// Reads the complete HAPI array attribute.
    pub fn get(&self) -> Result<JaggedArrayData<T>> {
        let info = &self.handle.info;
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

    /// Replaces the complete HAPI array attribute.
    ///
    /// The number of sizes must equal the cached attribute count. The
    /// [`JaggedArrayData`] constructor additionally guarantees that sizes are
    /// nonnegative and sum to the flattened data length.
    pub fn set(&self, values: &JaggedArrayData<T>) -> Result<()> {
        let info = &self.handle.info;
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
/// A typed string attribute handle.
///
/// Use [`Fixed`] for ordinary string tuples and [`Jagged`] for HAPI string
/// array attributes.
pub struct StringAttribute<S: AttributeShape> {
    handle: Handle,
    marker: PhantomData<S>,
}
#[derive(Debug, Clone)]
/// A typed JSON dictionary attribute handle.
///
/// Use [`Fixed`] for ordinary dictionary tuples and [`Jagged`] for HAPI
/// dictionary array attributes.
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
                    handle: Handle::new(name, info, node, part_id),
                    marker: PhantomData,
                }
            }
            /// Returns the currently cached metadata.
            #[must_use]
            pub fn info(&self) -> &AttributeInfo {
                &self.handle.info
            }
            /// Returns the attribute name.
            #[must_use]
            pub fn name(&self) -> &CStr {
                &self.handle.name
            }
            /// Returns the HAPI part to which this handle is bound.
            #[must_use]
            pub fn part_id(&self) -> i32 {
                self.handle.part_id
            }
            /// Returns the element owner of this attribute.
            #[must_use]
            pub fn owner(&self) -> AttributeOwner {
                self.handle.owner
            }
            /// Returns the HAPI storage type.
            #[must_use]
            pub fn storage(&self) -> StorageType {
                self.handle.info.storage()
            }
            /// Deletes this attribute from its bound node and part.
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

impl StringAttribute<Fixed> {
    /// Refreshes cached metadata, requiring fixed string storage.
    pub fn refresh(&mut self) -> Result<()> {
        self.handle.refresh(StorageType::String)
    }
}

impl StringAttribute<Jagged> {
    /// Refreshes cached metadata, requiring jagged string storage.
    pub fn refresh(&mut self) -> Result<()> {
        self.handle.refresh(StorageType::StringArray)
    }
}

impl DictionaryAttribute<Fixed> {
    /// Refreshes cached metadata, requiring fixed dictionary storage.
    pub fn refresh(&mut self) -> Result<()> {
        self.handle.refresh(StorageType::Dictionary)
    }
}

impl DictionaryAttribute<Jagged> {
    /// Refreshes cached metadata, requiring jagged dictionary storage.
    pub fn refresh(&mut self) -> Result<()> {
        self.handle.refresh(StorageType::DictionaryArray)
    }
}

macro_rules! fixed_string_impl {
    ($type:ident, $storage:expr, $dictionary:expr) => {
        impl $type<Fixed> {
            /// Reads the complete fixed-tuple attribute.
            pub fn get(&self) -> Result<StringArray> {
                let info = &self.handle.info;
                crate::ffi::get_string_attribute_data(
                    &self.handle.node,
                    self.handle.part_id,
                    &self.handle.name,
                    &info.0,
                    $dictionary,
                    0,
                    info.count(),
                )
            }
            /// Reads a range of owned elements as flattened string tuples.
            pub fn get_range(&self, range: Range<usize>) -> Result<StringArray> {
                let (start, count, _) = self.handle.checked_fixed_range(range)?;
                if count == 0 {
                    return Ok(StringArray::empty());
                }
                crate::ffi::get_string_attribute_data(
                    &self.handle.node,
                    self.handle.part_id,
                    &self.handle.name,
                    &self.handle.info.0,
                    $dictionary,
                    start,
                    count,
                )
            }
            /// Replaces the complete fixed-tuple attribute.
            ///
            /// The value count must equal `count * tuple_size`.
            pub fn set(&self, values: &[impl AsRef<CStr>]) -> Result<()> {
                let info = &self.handle.info;
                let expected = Handle::expected_fixed_len(info)?;
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
                    0,
                    info.count(),
                )
            }
            /// Replaces a range of owned elements from flattened string tuples.
            pub fn set_range(
                &self,
                range: Range<usize>,
                values: &[impl AsRef<CStr>],
            ) -> Result<()> {
                let (start, count, expected) = self.handle.checked_fixed_range(range)?;
                if values.len() != expected {
                    return Err(HapiError::Internal(format!(
                        "attribute {:?} range needs {expected} strings, got {}",
                        self.handle.name,
                        values.len()
                    )));
                }
                if count == 0 {
                    return Ok(());
                }
                let ptrs: Vec<_> = values.iter().map(|v| v.as_ref().as_ptr()).collect();
                crate::ffi::set_string_attribute_data(
                    &self.handle.node,
                    self.handle.part_id,
                    &self.handle.name,
                    &self.handle.info.0,
                    &ptrs,
                    $dictionary,
                    start,
                    count,
                )
            }
        }
    };
}
fixed_string_impl!(StringAttribute, StorageType::String, false);
fixed_string_impl!(DictionaryAttribute, StorageType::Dictionary, true);

impl StringAttribute<Fixed> {
    /// Assigns one string value to every element of the attribute.
    pub fn set_unique(&self, value: &CStr) -> Result<()> {
        let info = &self.handle.info;
        crate::ffi::set_string_unique_attribute_data(
            &self.handle.node,
            self.handle.part_id,
            &self.handle.name,
            &info.0,
            value,
        )
    }
    /// Sets the complete attribute using a table of unique strings and indices.
    ///
    /// `indices` must contain `count * tuple_size` entries. Each index is
    /// interpreted by HAPI as an entry in `values`.
    pub fn set_indexed(&self, values: &[impl AsRef<CStr>], indices: &[i32]) -> Result<()> {
        let info = &self.handle.info;
        let expected = Handle::expected_fixed_len(info)?;
        if indices.len() != expected {
            return Err(HapiError::Internal(format!(
                "attribute {:?} needs {expected} indices, got {}",
                self.handle.name,
                indices.len()
            )));
        }
        validate_string_indices(values.len(), indices)?;
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
            /// Reads the complete string or dictionary array attribute.
            pub fn get(&self) -> Result<StringJaggedArrayData> {
                let info = &self.handle.info;
                let (values, sizes) = crate::ffi::get_string_jagged_attribute_data(
                    &self.handle.node,
                    self.handle.part_id,
                    &self.handle.name,
                    &info.0,
                    $dictionary,
                )?;
                StringJaggedArrayData::from_hapi(values, sizes)
            }
            /// Replaces the complete string or dictionary array attribute.
            ///
            /// Sizes must be nonnegative, sum to `values.len()`, and contain
            /// one entry for every owned geometry element.
            pub fn set(&self, values: &[impl AsRef<CStr>], sizes: &[i32]) -> Result<()> {
                let info = &self.handle.info;
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
/// Exhaustive runtime representation of every supported HAPI attribute storage.
///
/// Returned by [`crate::geometry::Geometry::get_attribute`] when the caller
/// does not know the storage in advance. Match a variant to obtain its typed
/// handle, or use the common identity and deletion methods directly.
pub enum AnyAttribute {
    /// A fixed `i32` attribute.
    Int(Attribute<i32, Fixed>),
    /// A fixed `i64` attribute.
    Int64(Attribute<i64, Fixed>),
    /// A fixed `f32` attribute.
    Float(Attribute<f32, Fixed>),
    /// A fixed `f64` attribute.
    Float64(Attribute<f64, Fixed>),
    /// A fixed string attribute.
    String(StringAttribute<Fixed>),
    /// A fixed `u8` attribute.
    Uint8(Attribute<u8, Fixed>),
    /// A fixed `i8` attribute.
    Int8(Attribute<i8, Fixed>),
    /// A fixed `i16` attribute.
    Int16(Attribute<i16, Fixed>),
    /// A jagged `i32` array attribute.
    IntArray(Attribute<i32, Jagged>),
    /// A jagged `i64` array attribute.
    Int64Array(Attribute<i64, Jagged>),
    /// A jagged `f32` array attribute.
    FloatArray(Attribute<f32, Jagged>),
    /// A jagged `f64` array attribute.
    Float64Array(Attribute<f64, Jagged>),
    /// A jagged string array attribute.
    StringArray(StringAttribute<Jagged>),
    /// A jagged `u8` array attribute.
    Uint8Array(Attribute<u8, Jagged>),
    /// A jagged `i8` array attribute.
    Int8Array(Attribute<i8, Jagged>),
    /// A jagged `i16` array attribute.
    Int16Array(Attribute<i16, Jagged>),
    /// A fixed JSON dictionary attribute.
    Dictionary(DictionaryAttribute<Fixed>),
    /// A jagged JSON dictionary array attribute.
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
    /// Returns the attribute name, replacing invalid UTF-8 if necessary.
    #[must_use]
    pub fn name(&self) -> Cow<'_, str> {
        self.handle().name.to_string_lossy()
    }
    /// Returns the currently cached metadata.
    #[must_use]
    pub fn info(&self) -> &AttributeInfo {
        &self.handle().info
    }
    /// Returns the HAPI storage represented by the enum variant.
    #[must_use]
    pub fn storage(&self) -> StorageType {
        self.handle().info.storage()
    }
    /// Returns the HAPI part to which this handle is bound.
    #[must_use]
    pub fn part_id(&self) -> i32 {
        self.handle().part_id
    }
    /// Returns the element owner of this attribute.
    #[must_use]
    pub fn owner(&self) -> AttributeOwner {
        self.handle().owner
    }
    /// Refreshes the variant's cached metadata from HAPI.
    ///
    /// The enum variant and cache remain unchanged if the attribute is missing
    /// or its storage no longer matches the typed handle.
    pub fn refresh(&mut self) -> Result<()> {
        match self {
            Self::Int(v) => v.refresh(),
            Self::Int64(v) => v.refresh(),
            Self::Float(v) => v.refresh(),
            Self::Float64(v) => v.refresh(),
            Self::String(v) => v.refresh(),
            Self::Uint8(v) => v.refresh(),
            Self::Int8(v) => v.refresh(),
            Self::Int16(v) => v.refresh(),
            Self::IntArray(v) => v.refresh(),
            Self::Int64Array(v) => v.refresh(),
            Self::FloatArray(v) => v.refresh(),
            Self::Float64Array(v) => v.refresh(),
            Self::StringArray(v) => v.refresh(),
            Self::Uint8Array(v) => v.refresh(),
            Self::Int8Array(v) => v.refresh(),
            Self::Int16Array(v) => v.refresh(),
            Self::Dictionary(v) => v.refresh(),
            Self::DictionaryArray(v) => v.refresh(),
        }
    }
    /// Deletes this attribute from its bound node and part.
    pub fn delete(self) -> Result<()> {
        let h = self.handle();
        crate::ffi::delete_attribute(&h.node, h.part_id, &h.name, &h.info.0)
    }
}
