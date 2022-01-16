#![allow(unused)]
use super::array::{DataArray, StringMultiArray};
use crate::errors::Result;
pub use crate::ffi::enums::StorageType;
pub use crate::ffi::AttributeInfo;
use crate::node::HoudiniNode;
use crate::stringhandle::StringArray;
use duplicate::duplicate;
use std::borrow::Cow;
use std::ffi::{CStr, CString};

use std::any::Any;
use std::marker::PhantomData;

pub(crate) struct _NumericAttrData<T: AttribStorage> {
    pub(crate) info: AttributeInfo,
    pub(crate) name: CString,
    pub(crate) node: HoudiniNode,
    pub(crate) _m: PhantomData<T>,
}

pub(crate) struct _StringAttrData {
    pub(crate) info: AttributeInfo,
    pub(crate) name: CString,
    pub(crate) node: HoudiniNode,
}

pub struct NumericAttr<T: AttribStorage>(pub(crate) _NumericAttrData<T>);

// TODO: Special case for P attribute maybe?
pub struct NumericArrayAttr<T: AttribStorage>(pub(crate) _NumericAttrData<T>);

pub struct StringAttr(pub(crate) _StringAttrData);

pub struct StringArrayAttr(pub(crate) _StringAttrData);

pub trait AttribStorage {
    fn storage() -> StorageType;
}

macro_rules! impl_storage {
    ($tp:ty, $st:expr) => {
        impl AttribStorage for $tp {
            fn storage() -> StorageType {
                $st
            }
        }
    };
}

impl_storage!(i8, StorageType::Int8);
impl_storage!(u8, StorageType::Uint8);
impl_storage!(i16, StorageType::Int16);
impl_storage!(i32, StorageType::Int);
impl_storage!(i64, StorageType::Int64);
impl_storage!(f32, StorageType::Float);
impl_storage!(f64, StorageType::Float64);

impl<T: AttribAccess> NumericArrayAttr<T>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    pub(crate) fn new(
        name: CString,
        info: AttributeInfo,
        node: HoudiniNode,
    ) -> NumericArrayAttr<T> {
        NumericArrayAttr(_NumericAttrData {
            info,
            name,
            node,
            _m: Default::default(),
        })
    }
    pub fn boxed(self) -> Box<dyn AnyAttribWrapper> {
        Box::new(self)
    }
    // TODO: I like "read" more than "get"
    pub fn get(&self, part_id: i32) -> Result<DataArray<T>> {
        T::get_array(&self.0.name, &self.0.node, part_id, &self.0.info)
    }
    pub fn set(&self, part_id: i32, values: &DataArray<T>) -> Result<()> {
        T::set_array(&self.0.name, &self.0.node, part_id, &self.0.info, values)
    }
}

impl<T: AttribAccess> NumericAttr<T> {
    pub(crate) fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> NumericAttr<T> {
        NumericAttr(_NumericAttrData {
            info,
            name,
            node,
            _m: Default::default(),
        })
    }
    pub fn boxed(self) -> Box<dyn AnyAttribWrapper> {
        Box::new(self)
    }
    pub fn get(&self, part_id: i32) -> Result<Vec<T>> {
        T::get(&self.0.name, &self.0.node, part_id, &self.0.info)
    }
    pub fn set(&self, part_id: i32, values: &[T]) -> Result<()> {
        T::set(&self.0.name, &self.0.node, part_id, &self.0.info, values)
    }
}

impl StringAttr {
    pub fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> StringAttr {
        StringAttr(_StringAttrData { info, name, node })
    }
    pub fn boxed(self) -> Box<dyn AnyAttribWrapper> {
        Box::new(self)
    }
    fn get(&self, part_id: i32) -> Result<StringArray> {
        super::bindings::get_attribute_string_data(
            &self.0.node,
            part_id,
            self.0.name.as_c_str(),
            &self.0.info.inner,
        )
    }
    pub fn set(&self, part_id: i32, values: &[&str]) -> Result<()> {
        let cstr: std::result::Result<Vec<CString>, std::ffi::NulError> =
            values.iter().map(|s| CString::new(*s)).collect();
        let cstr = cstr?;
        let mut ptrs: Vec<&CStr> = cstr.iter().map(|cs| cs.as_c_str()).collect();
        super::bindings::set_attribute_string_data(
            &self.0.node,
            part_id,
            self.0.name.as_c_str(),
            &self.0.info.inner,
            ptrs.as_ref(),
        )
    }
}

impl StringArrayAttr {
    pub fn new(name: CString, info: AttributeInfo, node: HoudiniNode) -> StringArrayAttr {
        StringArrayAttr(_StringAttrData { info, name, node })
    }
    pub fn boxed(self) -> Box<dyn AnyAttribWrapper> {
        Box::new(self)
    }
    fn get(&self, part_id: i32) -> Result<StringMultiArray> {
        todo!()
    }
    fn set(&self, part_id: i32, values: &[&[&str]]) -> Result<()> {
        todo!()
    }
}

// calls to ffi on Rust type
pub trait AttribAccess: Sized + AttribStorage + 'static
// where
//     [Self]: ToOwned<Owned = Vec<Self>>,
{
    fn get(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Vec<Self>>;
    fn set(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        values: &[Self],
    ) -> Result<()>;
    fn get_array<'a>(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<DataArray<'a, Self>>
    where
        [Self]: ToOwned<Owned = Vec<Self>>;
    fn set_array<'a>(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        data: &DataArray<'a, Self>,
    ) -> Result<()>
    where
        [Self]: ToOwned<Owned = Vec<Self>>;
}

pub trait AsAttribute {
    fn info(&self) -> &AttributeInfo;
    fn storage(&self) -> StorageType;
    fn name(&self) -> Cow<str>;
}

impl<T: AttribStorage> AsAttribute for NumericAttr<T> {
    fn info(&self) -> &AttributeInfo {
        &self.0.info
    }
    fn storage(&self) -> StorageType {
        T::storage()
    }

    fn name(&self) -> Cow<str> {
        self.0.name.to_string_lossy()
    }
}

impl<T: AttribStorage> AsAttribute for NumericArrayAttr<T> {
    fn info(&self) -> &AttributeInfo {
        &self.0.info
    }
    fn storage(&self) -> StorageType {
        T::storage()
    }
    fn name(&self) -> Cow<str> {
        self.0.name.to_string_lossy()
    }
}

impl AsAttribute for StringAttr {
    fn info(&self) -> &AttributeInfo {
        &self.0.info
    }

    fn storage(&self) -> StorageType {
        StorageType::String
    }

    fn name(&self) -> Cow<str> {
        self.0.name.to_string_lossy()
    }
}

impl AsAttribute for StringArrayAttr {
    fn info(&self) -> &AttributeInfo {
        &self.0.info
    }

    fn storage(&self) -> StorageType {
        StorageType::StringArray
    }

    fn name(&self) -> Cow<str> {
        self.0.name.to_string_lossy()
    }
}

impl<T: Sized + AttribStorage + 'static> AttribAccess for T
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    fn get(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<Vec<Self>> {
        todo!()
    }
    fn set(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        values: &[Self],
    ) -> Result<()> {
        todo!()
    }
    fn get_array<'a>(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<DataArray<'a, Self>> {
        todo!()
    }
    fn set_array<'a>(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        data: &DataArray<'a, Self>,
    ) -> Result<()> {
        todo!()
    }
}

// object safe trait
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
        self.0.name()
    }
    pub fn storage(&self) -> StorageType {
        self.0.storage()
    }
}
