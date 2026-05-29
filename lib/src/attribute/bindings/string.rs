#[cfg(feature = "async-cooking")]
use crate::attribute::JobId;
#[cfg(feature = "async-cooking")]
use crate::attribute::async_::AsyncAttribResult;
use crate::attribute::{AttributeInfo, StringMultiArray};
use crate::errors::Result;
use crate::node::HoudiniNode;
use crate::raw;
use crate::session::StringArray;
use crate::stringhandle::StringHandle;
use crate::utils::{i32_to_usize, i64_to_i32_clamped, i64_to_usize, uzize_to_i32};
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
#[cfg(feature = "async-cooking")]
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
            std::ptr::from_ref(info).cast_mut(),
            data.as_mut_ptr().cast::<raw::HAPI_StringHandle>(),
            i64_to_i32_clamped(info.totalArrayElements),
            sizes.as_mut_ptr(),
            0,
            info.count,
            &raw mut job_id,
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
#[cfg(feature = "async-cooking")]
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
            i64_to_i32_clamped(info.totalArrayElements),
            sizes.as_ptr(),
            0,
            info.count,
            &raw mut job_id,
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
            std::ptr::from_ref(attr_info),
            array.as_mut_ptr(),
            0,
            uzize_to_i32(array.len()),
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
        let mut data_array = vec![StringHandle(0); i64_to_usize(info.total_array_elements())];
        let mut sizes_fixed_array = vec![0; i32_to_usize(info.count())];
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            info.ptr().cast_mut(),
            data_array.as_mut_ptr().cast::<raw::HAPI_StringHandle>(),
            i64_to_i32_clamped(info.total_array_elements()),
            sizes_fixed_array.as_mut_ptr(),
            0,
            uzize_to_i32(sizes_fixed_array.len()),
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
            std::ptr::from_ref(info).cast_mut(),
            data.as_mut_ptr(),
            uzize_to_i32(data.len()),
            sizes.as_ptr(),
            0,
            uzize_to_i32(sizes.len()),
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
        let attr_info = std::ptr::from_ref(attr_info).cast_mut();
        raw::_get_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            handles.as_mut_ptr().cast::<raw::HAPI_StringHandle>(),
            0,
            uzize_to_i32(handles.len()),
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
#[cfg(feature = "async-cooking")]
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
            std::ptr::from_ref(info),
            data.as_ptr().cast_mut(),
            0,
            info.count,
            &raw mut job_id,
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
#[cfg(feature = "async-cooking")]
pub(crate) fn _get_async_rust_fn(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &raw::HAPI_AttributeInfo,
) -> Result<AsyncAttribResult<StringHandle>> {
    unsafe {
        let buffer_size = (attr_info.count * attr_info.tupleSize) as usize;
        let mut data: Vec<StringHandle> = Vec::with_capacity(buffer_size);
        let session = node.session.clone();
        // SAFETY: Most likely an error in C API, it should not modify the info object,
        // but for some reason it wants a mut pointer
        let attr_info = std::ptr::from_ref(attr_info).cast_mut();
        let mut job_id = -1;
        raw::_get_async_ffi_fn(
            session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            data.as_mut_ptr().cast::<raw::HAPI_StringHandle>(),
            0,
            uzize_to_i32(buffer_size),
            &raw mut job_id,
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
            std::ptr::from_ref(info),
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
#[cfg(feature = "async-cooking")]
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
            std::ptr::from_ref(info),
            data,
            info.tupleSize,
            0,
            info.count,
            &raw mut job_id,
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
            uzize_to_i32(data.len()),
            indices.as_ptr(),
            0,
            uzize_to_i32(indices.len()),
        )
        .check_err(
            &node.session,
            || stringify!(Calling HAPI_SetAttributeIndexedStringData),
        )
    }
}

#[cfg(feature = "async-cooking")]
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
            uzize_to_i32(data.len()),
            indices.as_ptr(),
            0,
            uzize_to_i32(indices.len()),
            &raw mut job_id,
        )
        .check_err(
            &node.session,
            || stringify!(Calling HAPI_SetAttributeIndexedStringData),
        )?;

        Ok(job_id)
    }
}
