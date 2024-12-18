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
mod bindings;

use crate::errors::Result;
pub use crate::ffi::enums::StorageType;
pub use crate::ffi::AttributeInfo;
use crate::node::HoudiniNode;
use crate::stringhandle::StringArray;
pub use array::*;
pub use bindings::AttribAccess;
use std::any::Any;
use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;

impl StorageType {
    // Helper for matching array types to actual data type,
    // e.g StorageType::Array is actually an array of StorageType::Int,
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
    /// Read the attribute data into a provided buffer. The buffer will be auto-resized
    /// from the attribute info.
    pub fn read_into(&self, part_id: i32, buffer: &mut Vec<T>) -> Result<()> {
        debug_assert_eq!(self.0.info.storage(), T::storage());
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
            self.0.info.count(),
        )
    }
}

impl StringAttr {
    pub fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> StringAttr {
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
    pub fn set_unique(&self, part: i32, value: &str) -> Result<()> {
        let value = CString::new(value)?;
        unsafe {
            bindings::set_attribute_string_unique_data(
                &self.0.node,
                self.0.name.as_c_str(),
                &self.0.info.0,
                part,
                value.as_ptr(),
            )
        }
    }
}

impl StringArrayAttr {
    pub fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> StringArrayAttr {
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
    pub fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> Self {
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
    pub fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> Self {
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
pub trait AsAttribute {
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
