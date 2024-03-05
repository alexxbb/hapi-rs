use super::array::{DataArray, StringMultiArray};
use crate::ffi::raw;
use crate::ffi::raw::{HAPI_AttributeInfo, HAPI_StringHandle, StorageType};
use crate::ffi::AttributeInfo;
use crate::stringhandle::{StringArray, StringHandle};
use crate::{node::HoudiniNode, Result};
use duplicate::duplicate_item;
use std::ffi::CStr;

pub trait AttribAccess: Sized + 'static {
    fn storage() -> StorageType;
    fn storage_array() -> StorageType;
    fn get(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part_id: i32,
        buffer: &mut Vec<Self>,
    ) -> Result<()>;
    fn set(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part_id: i32,
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

#[duplicate_item(
[
_val_type [u8]
_storage [StorageType::Uint8]
_storage_array [StorageType::Uint8Array]
_get [HAPI_GetAttributeUInt8Data]
_set [HAPI_SetAttributeUInt8Data]
_get_array [HAPI_GetAttributeUInt8ArrayData]
_set_array [HAPI_SetAttributeUInt8ArrayData]
]
[
_val_type [i8]
_storage [StorageType::Int8]
_storage_array [StorageType::Int8Array]
_get [HAPI_GetAttributeInt8Data]
_set [HAPI_SetAttributeInt8Data]
_get_array [HAPI_GetAttributeInt8ArrayData]
_set_array [HAPI_SetAttributeInt8ArrayData]
]
[
_val_type [i16]
_storage [StorageType::Int16]
_storage_array [StorageType::Int16Array]
_get [HAPI_GetAttributeInt16Data]
_set [HAPI_SetAttributeInt16Data]
_get_array [HAPI_GetAttributeInt16ArrayData]
_set_array [HAPI_SetAttributeInt16ArrayData]
]
[
_val_type [i32]
_storage [StorageType::Int]
_storage_array [StorageType::IntArray]
_get [HAPI_GetAttributeIntData]
_set [HAPI_SetAttributeIntData]
_get_array [HAPI_GetAttributeIntArrayData]
_set_array [HAPI_SetAttributeIntArrayData]
]
[
_val_type [i64]
_storage [StorageType::Int64]
_storage_array [StorageType::Int64Array]
_get [HAPI_GetAttributeInt64Data]
_set [HAPI_SetAttributeInt64Data]
_get_array [HAPI_GetAttributeInt64ArrayData]
_set_array [HAPI_SetAttributeInt64ArrayData]
]
[
_val_type [f32]
_storage [StorageType::Float]
_storage_array [StorageType::FloatArray]
_get [HAPI_GetAttributeFloatData]
_set [HAPI_SetAttributeFloatData]
_get_array [HAPI_GetAttributeFloatArrayData]
_set_array [HAPI_SetAttributeFloatArrayData]
]
[
_val_type [f64]
_storage [StorageType::Float64]
_storage_array [StorageType::Float64Array]
_get [HAPI_GetAttributeFloat64Data]
_set [HAPI_SetAttributeFloat64Data]
_get_array [HAPI_GetAttributeFloat64ArrayData]
_set_array [HAPI_SetAttributeFloat64ArrayData]
]
)]
impl AttribAccess for _val_type {
    fn storage() -> StorageType {
        _storage
    }
    fn storage_array() -> StorageType {
        _storage_array
    }
    fn get(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        buffer: &mut Vec<Self>,
    ) -> Result<()> {
        debug_assert!(node.is_valid()?);
        buffer.resize(
            (info.inner.count * info.inner.tupleSize) as usize,
            _val_type::default(),
        );
        unsafe {
            // SAFETY: Most likely an error in C API, it should not modify the info object,
            // but for some reason it wants a mut pointer
            let attr_info = &info.inner as *const _ as *mut HAPI_AttributeInfo;
            raw::_get(
                node.session.ptr(),
                node.handle.0,
                part,
                name.as_ptr(),
                attr_info,
                -1,
                buffer.as_mut_ptr(),
                0,
                info.inner.count,
            )
            .check_err(&node.session, || stringify!(Calling _get))
        }
    }
    fn set(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        data: &[_val_type],
        start: i32,
        len: i32,
    ) -> Result<()> {
        unsafe {
            debug_assert!(node.is_valid()?);
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
            .check_err(&node.session, || stringify!(Calling _set))
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
        debug_assert!(node.is_valid()?);
        let mut data = vec![_val_type::default(); info.inner.totalArrayElements as usize];
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
                info.inner.count,
            )
            .check_err(&node.session, || stringify!(Calling _get_array))?;
        }

        Ok(DataArray::new_owned(data, sizes))
    }
    fn set_array(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        data: &[_val_type],
        sizes: &[i32],
    ) -> Result<()>
    where
        [Self]: ToOwned<Owned = Vec<Self>>,
    {
        debug_assert!(node.is_valid()?);
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
                info.inner.count,
            )
            .check_err(&node.session, || stringify!(Calling _set_array))?;
        }

        Ok(())
    }
}

#[duplicate_item(
[
_rust_fn [get_attribute_string_data]
_ffi_fn [HAPI_GetAttributeStringData]
]

[
_rust_fn [get_attribute_dictionary_data]
_ffi_fn [HAPI_GetAttributeDictionaryData]
]
)]
pub(crate) fn _rust_fn(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &HAPI_AttributeInfo,
) -> Result<StringArray> {
    debug_assert!(node.is_valid()?);
    unsafe {
        let mut handles = Vec::new();
        let count = attr_info.count;
        handles.resize((count * attr_info.tupleSize) as usize, StringHandle(0));
        // SAFETY: Most likely an error in C API, it should not modify the info object,
        // but for some reason it wants a mut pointer
        let attr_info = attr_info as *const _ as *mut HAPI_AttributeInfo;
        let handles_ptr = handles.as_mut_ptr() as *mut HAPI_StringHandle;
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            handles_ptr,
            0,
            count,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))?;
        crate::stringhandle::get_string_array(&handles, &node.session)
    }
}

#[duplicate_item(
[
_rust_fn [set_attribute_string_data]
_ffi_fn [HAPI_SetAttributeStringData]
]

[
_rust_fn [set_attribute_dictionary_data]
_ffi_fn [HAPI_SetAttributeDictionaryData]
]
)]
pub fn _rust_fn(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &HAPI_AttributeInfo,
    array: &mut [*const i8],
) -> Result<()> {
    debug_assert!(node.is_valid()?);
    unsafe {
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            array.as_mut_ptr(),
            0,
            array.len() as i32,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))
    }
}

pub fn get_attribute_string_array_data(
    node: &HoudiniNode,
    name: &CStr,
    part_id: i32,
    info: &raw::HAPI_AttributeInfo,
) -> Result<StringMultiArray> {
    debug_assert!(node.is_valid()?);
    unsafe {
        let mut data_array = vec![StringHandle(0); info.totalArrayElements as usize];
        let mut sizes_fixed_array = vec![0; info.count as usize];
        raw::HAPI_GetAttributeStringArrayData(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            info as *const _ as *mut _,
            data_array.as_mut_ptr() as *mut HAPI_StringHandle,
            info.totalArrayElements as i32,
            sizes_fixed_array.as_mut_ptr(),
            0,
            info.count,
        )
        .check_err(&node.session, || "Calling HAPI_GetAttributeStringArrayData")?;

        Ok(StringMultiArray {
            handles: data_array,
            sizes: sizes_fixed_array,
            session: node.session.clone(),
        })
    }
}

pub fn set_attribute_string_array_data(
    node: &HoudiniNode,
    name: &CStr,
    part_id: i32,
    info: &raw::HAPI_AttributeInfo,
    data: &[&CStr],
    sizes: &[i32],
) -> Result<()> {
    debug_assert!(node.is_valid()?);
    let mut array = Vec::from_iter(data.iter().map(|cs| cs.as_ptr()));
    unsafe {
        raw::HAPI_SetAttributeStringArrayData(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            info as *const _ as *mut _,
            array.as_mut_ptr(),
            info.totalArrayElements as i32,
            sizes.as_ptr(),
            0,
            info.count,
        )
        .check_err(&node.session, || "Calling HAPI_SetAttributeStringArrayData")
    }
}
