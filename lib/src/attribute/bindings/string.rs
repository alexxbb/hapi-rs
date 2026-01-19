use crate::attribute::async_::AsyncAttribResult;
use crate::attribute::{AttributeInfo, JobId, StringMultiArray};
use crate::errors::Result;
use crate::node::HoudiniNode;
use crate::raw;
use crate::session::StringArray;
use crate::stringhandle::StringHandle;
use duplicate::duplicate_item;
use std::ffi::CStr;

// STRING|DICT GET ARRAY ASYNC
#[duplicate_item(
[
_rust_fn [get_attribute_string_array_data_async]
_ffi_fn [HAPI_GetAttributeStringArrayDataAsync]
]

[
_rust_fn [get_attribute_dictionary_array_data_async]
_ffi_fn [HAPI_GetAttributeDictionaryArrayDataAsync]
]
)]

pub(crate) fn _rust_fn(
    node: &HoudiniNode,
    name: &CStr,
    part_id: i32,
    info: &raw::HAPI_AttributeInfo,
    data: &mut [StringHandle],
    sizes: &mut [i32],
) -> Result<JobId> {
    let mut job_id = -1;
    unsafe {
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            info as *const _ as *mut _,
            data.as_mut_ptr() as *mut raw::HAPI_StringHandle,
            info.totalArrayElements as i32,
            sizes.as_mut_ptr(),
            0,
            info.count,
            &mut job_id as *mut _,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))?;
        Ok(job_id)
    }
}

// STRING|DICT SET ARRAY ASYNC
#[duplicate_item(
[
_rust_fn [set_attribute_string_array_data_async]
_ffi_fn [HAPI_SetAttributeStringArrayDataAsync]
]

[
_rust_fn [set_attribute_dictionary_array_data_async]
_ffi_fn [HAPI_SetAttributeDictionaryArrayDataAsync]
]
)]

pub(crate) fn _rust_fn(
    node: &HoudiniNode,
    name: &CStr,
    part_id: i32,
    info: &raw::HAPI_AttributeInfo,
    data: &mut [*const i8],
    sizes: &[i32],
) -> Result<JobId> {
    let mut job_id = -1;
    unsafe {
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            info,
            data.as_mut_ptr(),
            info.totalArrayElements as i32,
            sizes.as_ptr(),
            0,
            info.count,
            &mut job_id as *mut _,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))?;
        Ok(job_id)
    }
}

// STRING|DICT SET
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
pub(crate) fn _rust_fn(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &raw::HAPI_AttributeInfo,
    array: &mut [*const i8],
) -> Result<()> {
    debug_assert!(node.is_valid()?);
    unsafe {
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info as *const _,
            array.as_mut_ptr(),
            0,
            array.len() as i32,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))
    }
}

// STRING|DICT GET ARRAY
#[duplicate_item(
[
_rust_fn [get_attribute_string_array_data]
_ffi_fn [HAPI_GetAttributeStringArrayData]
]

[
_rust_fn [get_attribute_dictionary_array_data]
_ffi_fn [HAPI_GetAttributeDictionaryArrayData]
]
)]
pub(crate) fn _rust_fn(
    node: &HoudiniNode,
    name: &CStr,
    part_id: i32,
    info: &AttributeInfo,
) -> Result<StringMultiArray> {
    debug_assert!(node.is_valid()?);
    unsafe {
        let mut data_array = vec![StringHandle(0); info.total_array_elements() as usize];
        let mut sizes_fixed_array = vec![0; info.count() as usize];
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            info.ptr() as *mut _,
            data_array.as_mut_ptr() as *mut raw::HAPI_StringHandle,
            info.total_array_elements() as i32,
            sizes_fixed_array.as_mut_ptr(),
            0,
            sizes_fixed_array.len() as i32,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))?;

        Ok(StringMultiArray {
            handles: data_array,
            sizes: sizes_fixed_array,
            session: debug_ignore::DebugIgnore(node.session.clone()),
        })
    }
}

// STRING|DICT SET ARRAY
#[duplicate_item(
[
_rust_fn [set_attribute_string_array_data]
_ffi_fn [HAPI_SetAttributeStringArrayData]
]

[
_rust_fn [set_attribute_dictionary_array_data]
_ffi_fn [HAPI_SetAttributeDictionaryArrayData]
]
)]
pub(crate) fn _rust_fn(
    node: &HoudiniNode,
    name: &CStr,
    info: &raw::HAPI_AttributeInfo,
    part_id: i32,
    data: &mut [*const i8],
    sizes: &[i32],
) -> Result<()> {
    debug_assert!(node.is_valid()?);
    unsafe {
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            info as *const _ as *mut _,
            data.as_mut_ptr(),
            data.len() as i32,
            sizes.as_ptr(),
            0,
            sizes.len() as i32,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))
    }
}

// STRING|DICT GET
#[duplicate_item(
[
_get_rust_fn [get_attribute_string_data]
_get_ffi_fn [HAPI_GetAttributeStringData]
]

[
_get_rust_fn [get_attribute_dictionary_data]
_get_ffi_fn [HAPI_GetAttributeDictionaryData]
]
)]
pub(crate) fn _get_rust_fn(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &raw::HAPI_AttributeInfo,
) -> Result<StringArray> {
    debug_assert!(node.is_valid()?);
    debug_assert!(attr_info.count > 0);
    unsafe {
        let mut handles = vec![StringHandle(0); attr_info.count as usize];
        // SAFETY: Most likely an error in C API, it should not modify the info object,
        // but for some reason it wants a mut pointer
        let attr_info = attr_info as *const _ as *mut raw::HAPI_AttributeInfo;
        raw::_get_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            handles.as_mut_ptr() as *mut raw::HAPI_StringHandle,
            0,
            handles.len() as i32,
        )
        .check_err(&node.session, || stringify!(Calling _get_ffi_fn))?;
        crate::stringhandle::get_string_array(&handles, &node.session)
    }
}

// STRING|DICT SET ASYNC
#[duplicate_item(
[
_rust_fn [set_attribute_string_data_async]
_ffi_fn [HAPI_SetAttributeStringDataAsync]
]

[
_rust_fn [set_attribute_dictionary_data_async]
_ffi_fn [HAPI_SetAttributeDictionaryDataAsync]
]
)]

pub(crate) fn _rust_fn(
    node: &HoudiniNode,
    name: &CStr,
    part_id: i32,
    info: &raw::HAPI_AttributeInfo,
    data: &mut [*const i8],
) -> Result<JobId> {
    let mut job_id = -1;
    unsafe {
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            info as *const _,
            data.as_ptr() as *mut _,
            0,
            info.count,
            &mut job_id as *mut _,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))?;
    }
    Ok(job_id)
}

// STRING|DICT GET ASYNC
#[duplicate_item(
[
_get_async_rust_fn [get_attribute_string_data_async]
_get_async_ffi_fn [HAPI_GetAttributeStringDataAsync]
]

[
_get_async_rust_fn [get_attribute_dictionary_data_async]
_get_async_ffi_fn [HAPI_GetAttributeDictionaryDataAsync]
]

)]
pub(crate) fn _get_async_rust_fn(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &raw::HAPI_AttributeInfo,
) -> Result<AsyncAttribResult<StringHandle>> {
    unsafe {
        let buffer_size = (attr_info.count * attr_info.tupleSize) as usize;
        let mut data = Vec::with_capacity(buffer_size);
        let session = node.session.clone();
        // SAFETY: Most likely an error in C API, it should not modify the info object,
        // but for some reason it wants a mut pointer
        let attr_info = attr_info as *const _ as *mut raw::HAPI_AttributeInfo;
        let mut job_id = -1;
        raw::_get_async_ffi_fn(
            session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            data.as_mut_ptr() as *mut raw::HAPI_StringHandle,
            0,
            buffer_size as i32,
            &mut job_id,
        )
        .check_err(&session, || stringify!(Calling _get_async_ffi_fn))?;
        Ok(AsyncAttribResult {
            job_id,
            data,
            size: buffer_size,
            session,
        })
    }
}

// STRING SET UNIQUE
#[duplicate_item(
[
_rust_fn [set_attribute_string_unique_data]
_ffi_fn [HAPI_SetAttributeStringUniqueData]
_val_type [*const ::std::os::raw::c_char]
]

)]

pub(crate) fn _rust_fn(
    node: &HoudiniNode,
    name: &CStr,
    info: &raw::HAPI_AttributeInfo,
    part: i32,
    data: _val_type,
) -> Result<()> {
    unsafe {
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part,
            name.as_ptr(),
            info as *const _,
            data,
            info.tupleSize,
            0,
            info.count,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))
    }
}

// STRING SET UNIQUE ASYNC
#[duplicate_item(
[
_rust_fn [set_attribute_string_unique_data_async]
_ffi_fn [HAPI_SetAttributeStringUniqueDataAsync]
_val_type [*const ::std::os::raw::c_char]
]

)]

pub(crate) fn _rust_fn(
    node: &HoudiniNode,
    name: &CStr,
    info: &raw::HAPI_AttributeInfo,
    part: i32,
    data: _val_type,
) -> Result<JobId> {
    let mut job_id = -1;
    unsafe {
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part,
            name.as_ptr(),
            info as *const _,
            data,
            info.tupleSize,
            0,
            info.count,
            &mut job_id as *mut _,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))?;
    }
    Ok(job_id)
}

pub(crate) fn set_attribute_indexed_string_data(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    info: &raw::HAPI_AttributeInfo,
    data: &mut [*const i8],
    indices: &[i32],
) -> Result<()> {
    unsafe {
        raw::HAPI_SetAttributeIndexedStringData(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            info,
            data.as_mut_ptr(),
            data.len() as i32,
            indices.as_ptr(),
            0,
            indices.len() as i32,
        )
        .check_err(
            &node.session,
            || stringify!(Calling HAPI_SetAttributeIndexedStringData),
        )
    }
}

pub(crate) fn set_attribute_indexed_string_data_async(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    info: &raw::HAPI_AttributeInfo,
    data: &mut [*const i8],
    indices: &[i32],
) -> Result<JobId> {
    let mut job_id = -1;
    unsafe {
        raw::HAPI_SetAttributeIndexedStringDataAsync(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            info,
            data.as_mut_ptr(),
            data.len() as i32,
            indices.as_ptr(),
            0,
            indices.len() as i32,
            &mut job_id as *mut _,
        )
        .check_err(
            &node.session,
            || stringify!(Calling HAPI_SetAttributeIndexedStringData),
        )?;

        Ok(job_id)
    }
}
