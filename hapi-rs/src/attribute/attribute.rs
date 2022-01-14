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

pub struct NumericAttr<T> {
    pub(crate) info: AttributeInfo,
    pub(crate) name: CString,
    pub(crate) node: HoudiniNode,
    pub(crate) _m: PhantomData<T>,
}

pub struct NumericArrayAttr<T> {
    pub(crate) info: AttributeInfo,
    pub(crate) name: CString,
    pub(crate) node: HoudiniNode,
    pub(crate) _m: PhantomData<T>,
}

macro_rules! box_attr {
    ($st:ident, $tp:ty, $info:ident, $name:ident, $node:ident) => {
        Box::new($st::<$tp> {
            $info,
            $name,
            $node,
            _m: std::marker::PhantomData,
        })
    };
}

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
pub(crate) use box_attr;

impl_storage!(i8, StorageType::Int8);
impl_storage!(u8, StorageType::Uint8);
impl_storage!(i16, StorageType::Int16);
impl_storage!(i32, StorageType::Int);
impl_storage!(i64, StorageType::Int64);
impl_storage!(f32, StorageType::Float);
impl_storage!(f64, StorageType::Float64);

impl<T: AttribAccess> NumericArrayAttr<T> {
    fn get(&self, part_id: i32) -> Result<DataArray<Vec<T>>> {
        T::get_array(&self.name, &self.node, part_id, &self.info)
    }
    fn set(&self, part_id: i32, values: &DataArray<&[T]>) -> Result<()> {
        T::set_array(&self.name, &self.node, part_id, &self.info, values)
    }
}

impl<T: AttribAccess> NumericAttr<T> {
    fn get(&self, part_id: i32) -> Result<Vec<T>> {
        T::get(&self.name, &self.node, part_id, &self.info)
    }
    fn set(&self, part_id: i32, values: &[T]) -> Result<()> {
        T::set(&self.name, &self.node, part_id, &self.info, values)
    }
}

pub trait AttribAccess: Sized + AttribStorage {
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
    fn get_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<DataArray<Vec<Self>>>;
    fn set_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        data: &DataArray<&[Self]>,
    ) -> Result<()>;
}

pub trait AsAttribute {
    fn info(&self) -> &AttributeInfo;
    fn storage(&self) -> StorageType;
    fn name(&self) -> Cow<str>;
}

impl<T: AttribStorage> AsAttribute for NumericAttr<T> {
    fn info(&self) -> &AttributeInfo {
        &self.info
    }
    fn storage(&self) -> StorageType {
        T::storage()
    }

    fn name(&self) -> Cow<str> {
        self.name.to_string_lossy()
    }
}
impl<T: AttribStorage> AsAttribute for NumericArrayAttr<T> {
    fn info(&self) -> &AttributeInfo {
        &self.info
    }
    fn storage(&self) -> StorageType {
        T::storage()
    }
    fn name(&self) -> Cow<str> {
        self.name.to_string_lossy()
    }
}

impl<T: Sized + AttribStorage> AttribAccess for T {
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
    fn get_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
    ) -> Result<DataArray<Vec<Self>>> {
        todo!()
    }
    fn set_array(
        name: &CStr,
        node: &'_ HoudiniNode,
        part_id: i32,
        info: &AttributeInfo,
        data: &DataArray<&[Self]>,
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
