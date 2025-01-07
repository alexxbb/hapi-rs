//! Geometry attributes access and iterators
//!
//! Geometry attributes of different types are represented as trait objects
//! and need to be downcast to concrete types
//!
//! ```
//!
//! use hapi_rs::session::new_in_process;
//! use hapi_rs::geometry::*;
//! use hapi_rs::attribute::*;
//! let session = new_in_process(None).unwrap();
//! let lib = session.load_asset_file("otls/hapi_geo.hda").unwrap();
//! let node = lib.try_create_first().unwrap();
//! let geo = node.geometry().unwrap().unwrap();
//! geo.node.cook_blocking().unwrap();
//! let attr_p = geo.get_attribute(0, AttributeOwner::Point, "P").unwrap().expect("P exists");
//! let attr_p = attr_p.downcast::<NumericAttr<f32>>().unwrap();
//! attr_p.get(0).expect("read_attribute");
//!
//! ```
mod array;
mod async_;
mod bindings;

use crate::errors::Result;
pub use crate::ffi::enums::StorageType;
pub use crate::ffi::AttributeInfo;
use crate::node::HoudiniNode;
use crate::stringhandle::{StringArray, StringHandle};
pub use array::*;
use async_::AsyncAttribResult;
use std::any::Any;
use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;

pub type JobId = i32;

mod private {
    pub trait Sealed {}
}
pub trait AttribAccess: private::Sealed + Clone + Default + Send + Sized + 'static {
    fn storage() -> StorageType;
    fn storage_array() -> StorageType;
    fn get(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part_id: i32,
        buffer: &mut Vec<Self>,
    ) -> Result<()>;
    fn get_async(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part_id: i32,
        buffer: &mut Vec<Self>,
    ) -> Result<JobId>;
    fn set(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part_id: i32,
        data: &[Self],
        start: i32,
        len: i32,
    ) -> Result<()>;

    fn set_unique(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part_id: i32,
        data: &[Self],
        start: i32,
    ) -> Result<()>;

    fn get_array(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
    ) -> Result<DataArray<'static, Self>>
    where
        [Self]: ToOwned<Owned = Vec<Self>>;

    fn get_array_async(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        data: &mut [Self],
        sizes: &mut [i32],
        part: i32,
    ) -> Result<JobId>;
    fn set_array(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        data: &[Self],
        sizes: &[i32],
    ) -> Result<()>
    where
        [Self]: ToOwned<Owned = Vec<Self>>;
}

macro_rules! impl_sealed {
    ($($x:ident),+ $(,)?) => {
        $(impl private::Sealed for $x {})+
    }
}

impl_sealed!(u8, i8, i16, i32, i64, f32, f64);

impl StorageType {
    // Helper for matching array types to actual data type,
    // e.g. StorageType::Array is actually an array of StorageType::Int,
    // StorageType::FloatArray is StorageType::Float
    pub(crate) fn type_matches(&self, other: StorageType) -> bool {
        use StorageType::*;
        match other {
            IntArray | Uint8Array | Int8Array | Int16Array | Int64Array => {
                matches!(*self, Int | Uint8 | Int16 | Int64)
            }
            FloatArray | Float64Array => matches!(*self, Float | Float64),
            StringArray => matches!(*self, StringArray),
            _st => matches!(*self, _st),
        }
    }
}

pub(crate) struct AttributeBundle {
    pub(crate) info: AttributeInfo,
    pub(crate) name: CString,
    pub(crate) node: HoudiniNode,
}

pub struct NumericAttr<T: AttribAccess>(pub(crate) AttributeBundle, PhantomData<T>);

pub struct NumericArrayAttr<T: AttribAccess>(pub(crate) AttributeBundle, PhantomData<T>);

pub struct StringAttr(pub(crate) AttributeBundle);

pub struct StringArrayAttr(pub(crate) AttributeBundle);

pub struct DictionaryAttr(pub(crate) AttributeBundle);

pub struct DictionaryArrayAttr(pub(crate) AttributeBundle);

impl<T: AttribAccess> NumericArrayAttr<T>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    pub(crate) fn new(
        name: CString,
        info: AttributeInfo,
        node: HoudiniNode,
    ) -> NumericArrayAttr<T> {
        NumericArrayAttr(AttributeBundle { info, name, node }, PhantomData)
    }
    pub fn get(&self, part_id: i32) -> Result<DataArray<T>> {
        debug_assert!(self.0.info.storage().type_matches(T::storage()));
        T::get_array(&self.0.name, &self.0.node, &self.0.info, part_id)
    }

    pub fn get_async(&self, part_id: i32) -> Result<(JobId, DataArray<T>)> {
        let info = &self.0.info;
        debug_assert!(info.storage().type_matches(T::storage()));
        let mut data = vec![T::default(); info.total_array_elements() as usize];
        let mut sizes = vec![0i32; info.count() as usize];
        let job_id = T::get_array_async(
            &self.0.name,
            &self.0.node,
            &info,
            &mut data,
            &mut sizes,
            part_id,
        )?;
        Ok((job_id, DataArray::new_owned(data, sizes)))
    }

    pub fn set(&self, part_id: i32, values: &DataArray<T>) -> Result<()> {
        debug_assert!(self.0.info.storage().type_matches(T::storage()));
        debug_assert_eq!(
            self.0.info.count(),
            values.sizes().len() as i32,
            "sizes array must be the same as AttributeInfo::count"
        );
        debug_assert_eq!(
            self.0.info.total_array_elements(),
            values.data().len() as i64,
            "data array must be the same as AttributeInfo::total_array_elements"
        );
        T::set_array(
            &self.0.name,
            &self.0.node,
            &self.0.info,
            part_id,
            values.data(),
            values.sizes(),
        )
    }
}

impl<T: AttribAccess> NumericAttr<T> {
    pub(crate) fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> NumericAttr<T> {
        NumericAttr(AttributeBundle { info, name, node }, PhantomData)
    }
    /// Get attribute value. Allocates a new vector on every call
    pub fn get(&self, part_id: i32) -> Result<Vec<T>> {
        debug_assert_eq!(self.0.info.storage(), T::storage());
        let mut buffer = vec![];
        T::get(
            &self.0.name,
            &self.0.node,
            &self.0.info,
            part_id,
            &mut buffer,
        )?;
        Ok(buffer)
    }
    /// Start filling a given buffer asynchronously and return a job id.
    /// It's important to keep the buffer alive until the job is complete
    pub fn read_async_into(&self, part_id: i32, buffer: &mut Vec<T>) -> Result<i32> {
        // TODO: Get an updated attribute info since point count can change between calls.
        // but there's looks like some use after free on the C side, when AttributeInfo gets
        // accessed after it gets dropped in Rust, so we can't get new AttributeInfo here.
        let info = &self.0.info;
        buffer.resize((info.count() * info.tuple_size()) as usize, T::default());
        T::get_async(&self.0.name, &self.0.node, &self.0.info, part_id, buffer)
    }

    pub fn get_async(&self, part_id: i32) -> Result<AsyncAttribResult<T>> {
        let info = &self.0.info;
        let size = (info.count() * info.tuple_size()) as usize;
        let mut data = Vec::<T>::with_capacity(size);
        let job_id = T::get_async(&self.0.name, &self.0.node, &info, part_id, &mut data)?;
        Ok(AsyncAttribResult {
            job_id,
            data,
            size,
            session: self.0.node.session.clone(),
        })
    }

    /// Read the attribute data into a provided buffer. The buffer will be auto-resized
    /// from the attribute info.
    pub fn read_into(&self, part_id: i32, buffer: &mut Vec<T>) -> Result<()> {
        debug_assert_eq!(self.0.info.storage(), T::storage());
        // Get an updated attribute info since point count can change between calls
        let info = AttributeInfo::new(&self.0.node, part_id, self.0.info.owner(), &self.0.name)?;
        T::get(&self.0.name, &self.0.node, &info, part_id, buffer)
    }
    pub fn set(&self, part_id: i32, values: &[T]) -> Result<()> {
        debug_assert_eq!(self.0.info.storage(), T::storage());
        T::set(
            &self.0.name,
            &self.0.node,
            &self.0.info,
            part_id,
            values,
            0,
            self.0.info.count().min(values.len() as i32),
        )
    }

    /// Set multiple attribute data to the same value.
    /// value length must be less or equal to attribute tuple size.
    pub fn set_unique(&self, part_id: i32, value: &[T]) -> Result<()> {
        debug_assert_eq!(self.0.info.storage(), T::storage());
        debug_assert!(value.len() <= self.0.info.tuple_size() as usize);
        T::set_unique(&self.0.name, &self.0.node, &self.0.info, part_id, value, 0)
    }
}

impl StringAttr {
    pub(crate) fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> StringAttr {
        StringAttr(AttributeBundle { info, name, node })
    }
    pub fn get(&self, part_id: i32) -> Result<StringArray> {
        debug_assert!(self.0.node.is_valid()?);
        bindings::get_attribute_string_data(
            &self.0.node,
            part_id,
            self.0.name.as_c_str(),
            &self.0.info.0,
        )
    }

    pub fn get_async(&self, part_id: i32) -> Result<AsyncAttribResult<StringHandle>> {
        bindings::get_attribute_string_data_async(
            &self.0.node,
            part_id,
            self.0.name.as_c_str(),
            &self.0.info.0,
        )
    }
    pub fn set(&self, part_id: i32, values: &[&str]) -> Result<()> {
        debug_assert!(self.0.node.is_valid()?);
        let cstr: std::result::Result<Vec<CString>, std::ffi::NulError> =
            values.iter().map(|s| CString::new(*s)).collect();
        let cstr = cstr?;
        let mut ptrs: Vec<*const i8> = cstr.iter().map(|cs| cs.as_ptr()).collect();
        bindings::set_attribute_string_data(
            &self.0.node,
            part_id,
            self.0.name.as_c_str(),
            &self.0.info.0,
            ptrs.as_mut(),
        )
    }
    pub fn set_cstr<'a>(&self, part_id: i32, values: impl Iterator<Item = &'a CStr>) -> Result<()> {
        let mut ptrs: Vec<*const i8> = values.map(|cs| cs.as_ptr()).collect();
        bindings::set_attribute_string_data(
            &self.0.node,
            part_id,
            self.0.name.as_c_str(),
            &self.0.info.0,
            ptrs.as_mut(),
        )
    }
    /// Set multiple attribute data to the same value.
    /// value length must be less or equal to attribute tuple size.
    pub fn set_unique(&self, part: i32, value: &str) -> Result<()> {
        let value = CString::new(value)?;
        bindings::set_attribute_string_unique_data(
            &self.0.node,
            self.0.name.as_c_str(),
            &self.0.info.0,
            part,
            value.as_ptr(),
        )
    }
}

impl StringArrayAttr {
    pub(crate) fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> StringArrayAttr {
        StringArrayAttr(AttributeBundle { info, name, node })
    }
    pub fn get(&self, part_id: i32) -> Result<StringMultiArray> {
        debug_assert!(self.0.node.is_valid()?);
        bindings::get_attribute_string_array_data(
            &self.0.node,
            self.0.name.as_c_str(),
            part_id,
            &self.0.info,
        )
    }

    pub fn get_async(&self, part_id: i32) -> Result<(JobId, StringMultiArray)> {
        let mut handles = vec![StringHandle(-1); self.0.info.total_array_elements() as usize];
        let mut sizes = vec![0; self.0.info.count() as usize];
        let job_id = bindings::get_attribute_string_array_data_async(
            &self.0.node,
            self.0.name.as_c_str(),
            part_id,
            &self.0.info.0,
            &mut handles,
            &mut sizes,
        )?;
        Ok((
            job_id,
            StringMultiArray {
                handles,
                sizes,
                session: self.0.node.session.clone(),
            },
        ))
    }
    pub fn set(&self, values: &[impl AsRef<str>], sizes: &[i32]) -> Result<()> {
        debug_assert!(self.0.node.is_valid()?);
        let cstr: std::result::Result<Vec<CString>, std::ffi::NulError> =
            values.iter().map(|s| CString::new(s.as_ref())).collect();
        let cstr = cstr?;
        let mut ptrs: Vec<_> = cstr.iter().map(|cs| cs.as_ptr()).collect();
        bindings::set_attribute_string_array_data(
            &self.0.node,
            self.0.name.as_c_str(),
            &self.0.info.0,
            ptrs.as_mut(),
            &sizes,
        )
    }
}

impl DictionaryAttr {
    pub(crate) fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> Self {
        DictionaryAttr(AttributeBundle { info, name, node })
    }

    pub fn get(&self, part_id: i32) -> Result<StringArray> {
        debug_assert!(self.0.node.is_valid()?);
        bindings::get_attribute_dictionary_data(
            &self.0.node,
            part_id,
            self.0.name.as_c_str(),
            &self.0.info.0,
        )
    }

    pub fn get_async(&self, part_id: i32) -> Result<AsyncAttribResult<StringHandle>> {
        bindings::get_attribute_dictionary_data_async(
            &self.0.node,
            part_id,
            self.0.name.as_c_str(),
            &self.0.info.0,
        )
    }

    /// Set dictionary attribute values where each string should be a JSON-encoded value.
    pub fn set(&self, part_id: i32, values: &[impl AsRef<str>]) -> Result<()> {
        debug_assert!(self.0.node.is_valid()?);
        let cstr: std::result::Result<Vec<CString>, std::ffi::NulError> =
            values.iter().map(|s| CString::new(s.as_ref())).collect();
        let cstr = cstr?;
        let mut cstrings: Vec<*const i8> = cstr.iter().map(|cs| cs.as_ptr()).collect();
        bindings::set_attribute_dictionary_data(
            &self.0.node,
            part_id,
            &self.0.name.as_c_str(),
            &self.0.info.0,
            cstrings.as_mut(),
        )
    }
}

impl DictionaryArrayAttr {
    pub(crate) fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> Self {
        DictionaryArrayAttr(AttributeBundle { info, name, node })
    }
    pub fn get(&self, part_id: i32) -> Result<StringMultiArray> {
        debug_assert!(self.0.node.is_valid()?);
        bindings::get_attribute_dictionary_array_data(
            &self.0.node,
            &self.0.name,
            part_id,
            &self.0.info,
        )
    }

    pub fn get_async(&self, part_id: i32) -> Result<(JobId, StringMultiArray)> {
        let mut handles = vec![StringHandle(-1); self.0.info.total_array_elements() as usize];
        let mut sizes = vec![0; self.0.info.count() as usize];
        let job_id = bindings::get_attribute_dictionary_array_data_async(
            &self.0.node,
            self.0.name.as_c_str(),
            part_id,
            &self.0.info.0,
            &mut handles,
            &mut sizes,
        )?;
        Ok((
            job_id,
            StringMultiArray {
                handles,
                sizes,
                session: self.0.node.session.clone(),
            },
        ))
    }
    pub fn set(&self, values: &[impl AsRef<str>], sizes: &[i32]) -> Result<()> {
        debug_assert!(self.0.node.is_valid()?);
        let cstrings: std::result::Result<Vec<CString>, std::ffi::NulError> =
            values.iter().map(|s| CString::new(s.as_ref())).collect();
        let cstrings = cstrings?;
        let mut ptrs: Vec<_> = cstrings.iter().map(|cs| cs.as_ptr()).collect();
        bindings::set_attribute_dictionary_array_data(
            &self.0.node,
            self.0.name.as_c_str(),
            &self.0.info.0,
            ptrs.as_mut(),
            &sizes,
        )
    }
}

#[doc(hidden)]
pub trait AsAttribute: Send {
    fn info(&self) -> &AttributeInfo;
    fn storage(&self) -> StorageType;
    fn boxed(self) -> Box<dyn AnyAttribWrapper>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
    fn name(&self) -> &CStr;
    fn node(&self) -> &HoudiniNode;
}

impl<T: AttribAccess> AsAttribute for NumericAttr<T> {
    fn info(&self) -> &AttributeInfo {
        &self.0.info
    }
    fn storage(&self) -> StorageType {
        T::storage()
    }

    fn name(&self) -> &CStr {
        &self.0.name
    }

    fn node(&self) -> &HoudiniNode {
        &self.0.node
    }
}

impl<T: AttribAccess> AsAttribute for NumericArrayAttr<T> {
    fn info(&self) -> &AttributeInfo {
        &self.0.info
    }
    fn storage(&self) -> StorageType {
        T::storage()
    }
    fn name(&self) -> &CStr {
        &self.0.name
    }
    fn node(&self) -> &HoudiniNode {
        &self.0.node
    }
}

impl AsAttribute for StringAttr {
    fn info(&self) -> &AttributeInfo {
        &self.0.info
    }

    fn storage(&self) -> StorageType {
        StorageType::String
    }

    fn name(&self) -> &CStr {
        &self.0.name
    }

    fn node(&self) -> &HoudiniNode {
        &self.0.node
    }
}

impl AsAttribute for StringArrayAttr {
    fn info(&self) -> &AttributeInfo {
        &self.0.info
    }

    fn storage(&self) -> StorageType {
        StorageType::StringArray
    }

    fn name(&self) -> &CStr {
        &self.0.name
    }

    fn node(&self) -> &HoudiniNode {
        &self.0.node
    }
}

impl AsAttribute for DictionaryAttr {
    fn info(&self) -> &AttributeInfo {
        &self.0.info
    }

    fn storage(&self) -> StorageType {
        StorageType::Dictionary
    }

    fn name(&self) -> &CStr {
        &self.0.name
    }

    fn node(&self) -> &HoudiniNode {
        &self.0.node
    }
}

impl AsAttribute for DictionaryArrayAttr {
    fn info(&self) -> &AttributeInfo {
        &self.0.info
    }

    fn storage(&self) -> StorageType {
        StorageType::DictionaryArray
    }

    fn name(&self) -> &CStr {
        &self.0.name
    }

    fn node(&self) -> &HoudiniNode {
        &self.0.node
    }
}

#[doc(hidden)]
pub trait AnyAttribWrapper: Any + AsAttribute {
    fn as_any(&self) -> &dyn Any;
}

impl<T: AsAttribute + 'static> AnyAttribWrapper for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct Attribute(Box<dyn AnyAttribWrapper>);

impl Attribute {
    pub(crate) fn new(attr_obj: Box<dyn AnyAttribWrapper>) -> Self {
        Attribute(attr_obj)
    }
    pub fn downcast<T: AnyAttribWrapper>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref::<T>()
    }
    pub fn name(&self) -> Cow<str> {
        self.0.name().to_string_lossy()
    }
    pub fn storage(&self) -> StorageType {
        self.0.storage()
    }
    pub fn info(&self) -> &AttributeInfo {
        self.0.info()
    }
    pub fn delete(self, part_id: i32) -> Result<()> {
        crate::ffi::delete_attribute(self.0.node(), part_id, self.0.name(), &self.0.info().0)
    }
}
