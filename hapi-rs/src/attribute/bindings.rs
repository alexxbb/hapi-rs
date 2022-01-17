use super::array::DataArray;
use crate::ffi::raw;
use crate::ffi::raw::{HAPI_AttributeInfo, StorageType};
use crate::ffi::AttributeInfo;
use crate::stringhandle::StringArray;
use crate::{node::HoudiniNode, Result};
use duplicate::duplicate;
use std::ffi::CStr;

pub trait AttribAccess: Sized + 'static {
    fn storage() -> StorageType;
    fn get(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        stride: i32,
        start: i32,
        len: i32,
    ) -> Result<Vec<Self>>;
    fn set(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        data: &[Self],
        start: i32,
        len: i32,
    ) -> Result<()>;
    fn get_array(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
    ) -> Result<DataArray<'static, Self>>
    where
        [Self]: ToOwned<Owned = Vec<Self>>;
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

#[duplicate(
[
_int_type [u8]
_storage [StorageType::Uint8]
_get [HAPI_GetAttributeUInt8Data]
_set [HAPI_SetAttributeUInt8Data]
_get_array [HAPI_GetAttributeUInt8ArrayData]
_set_array [HAPI_SetAttributeUInt8ArrayData]
]
[
_int_type [i8]
_storage [StorageType::Int8]
_get [HAPI_GetAttributeInt8Data]
_set [HAPI_SetAttributeInt8Data]
_get_array [HAPI_GetAttributeInt8ArrayData]
_set_array [HAPI_SetAttributeInt8ArrayData]
]
[
_int_type [i16]
_storage [StorageType::Int16]
_get [HAPI_GetAttributeInt16Data]
_set [HAPI_SetAttributeInt16Data]
_get_array [HAPI_GetAttributeInt16ArrayData]
_set_array [HAPI_SetAttributeInt16ArrayData]
]
[
_int_type [i32]
_storage [StorageType::Int]
_get [HAPI_GetAttributeIntData]
_set [HAPI_SetAttributeIntData]
_get_array [HAPI_GetAttributeIntArrayData]
_set_array [HAPI_SetAttributeIntArrayData]
]
[
_int_type [i64]
_storage [StorageType::Int64]
_get [HAPI_GetAttributeInt64Data]
_set [HAPI_SetAttributeInt64Data]
_get_array [HAPI_GetAttributeInt64ArrayData]
_set_array [HAPI_SetAttributeInt64ArrayData]
]
[
_int_type [f32]
_storage [StorageType::Float]
_get [HAPI_GetAttributeFloatData]
_set [HAPI_SetAttributeFloatData]
_get_array [HAPI_GetAttributeFloatArrayData]
_set_array [HAPI_SetAttributeFloatArrayData]
]
[
_int_type [f64]
_storage [StorageType::Float64]
_get [HAPI_GetAttributeFloat64Data]
_set [HAPI_SetAttributeFloat64Data]
_get_array [HAPI_GetAttributeFloat64ArrayData]
_set_array [HAPI_SetAttributeFloat64ArrayData]
]
)]
impl AttribAccess for _int_type {
    fn storage() -> StorageType {
        _storage
    }
    fn get(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        stride: i32,
        start: i32,
        len: i32,
    ) -> Result<Vec<_int_type>> {
        unsafe {
            let mut data_array = Vec::new();
            data_array.resize((len * info.inner.tupleSize) as usize, _int_type::default());
            // SAFETY: Most likely an error in C API, it should not modify the info object,
            // but for some reason it wants a mut pointer
            let attr_info = &info.inner as *const _ as *mut HAPI_AttributeInfo;
            // let mut data_array = vec![];
            raw::_get(
                node.session.ptr(),
                node.handle.0,
                part,
                name.as_ptr(),
                attr_info,
                stride,
                data_array.as_mut_ptr(),
                start,
                len,
            )
            .check_err(Some(&node.session))?;
            Ok(data_array)
        }
    }
    fn set(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        data: &[Self],
        start: i32,
        len: i32,
    ) -> Result<()> {
        unsafe {
            raw::_set(
                node.session.ptr(),
                node.handle.0,
                part,
                name.as_ptr(),
                &info.inner,
                data.as_ptr(),
                start,
                len,
            )
            .check_err(Some(&node.session))
        }
    }
    fn get_array(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
    ) -> Result<DataArray<'static, Self>>
    where
        [Self]: ToOwned<Owned = Vec<Self>>,
    {
        let mut data = vec![_int_type::default(); info.inner.totalArrayElements as usize];
        let mut sizes = vec![0; info.inner.count as usize];
        unsafe {
            raw::_get_array(
                node.session.ptr(),
                node.handle.0,
                part,
                name.as_ptr(),
                &info.inner as *const _ as *mut _,
                data.as_mut_ptr(),
                info.inner.totalArrayElements as i32,
                sizes.as_mut_ptr(),
                0,
                info.inner.count as i32,
            )
            .check_err(Some(&node.session))?;
        }

        Ok(DataArray::new_owned(data, sizes))
    }
    fn set_array(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        data: &[Self],
        sizes: &[i32],
    ) -> Result<()>
    where
        [Self]: ToOwned<Owned = Vec<Self>>,
    {
        unsafe {
            raw::_set_array(
                node.session.ptr(),
                node.handle.0,
                part,
                name.as_ptr(),
                &info.inner,
                data.as_ptr(),
                info.inner.totalArrayElements as i32,
                sizes.as_ptr(),
                0,
                info.inner.count as i32,
            )
            .check_err(Some(&node.session))?;
        }

        Ok(())
    }
}

pub fn get_attribute_string_data(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &HAPI_AttributeInfo,
) -> Result<StringArray> {
    unsafe {
        let mut handles = Vec::new();
        let count = attr_info.count;
        handles.resize((count * attr_info.tupleSize) as usize, 0);
        // SAFETY: Most likely an error in C API, it should not modify the info object,
        // but for some reason it wants a mut pointer
        let attr_info = attr_info as *const _ as *mut HAPI_AttributeInfo;
        raw::HAPI_GetAttributeStringData(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            handles.as_mut_ptr(),
            0,
            count,
        )
        .check_err(Some(&node.session))?;
        crate::stringhandle::get_string_array(&handles, &node.session)
    }
}

pub fn set_attribute_string_data(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &HAPI_AttributeInfo,
    array: &[&CStr],
) -> Result<()> {
    unsafe {
        let mut array = Vec::from_iter(array.iter().map(|cs| cs.as_ptr()));
        raw::HAPI_SetAttributeStringData(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            array.as_mut_ptr(),
            0,
            array.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}
