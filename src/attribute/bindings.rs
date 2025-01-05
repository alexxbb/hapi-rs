use super::array::{DataArray, StringMultiArray};
use crate::ffi::raw;
use crate::ffi::raw::{HAPI_AttributeInfo, HAPI_StringHandle, StorageType};
use crate::ffi::AttributeInfo;
use crate::session::Session;
use crate::stringhandle::{StringArray, StringHandle};
use crate::{node::HoudiniNode, Result};
use duplicate::duplicate_item;
use std::ffi::CStr;

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
    ) -> Result<i32>;
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

impl_sealed!(u8, i8, i16, u16, i32, i64, f32, f64);

#[duplicate_item(
[
_val_type [u8]
_storage [StorageType::Uint8]
_storage_array [StorageType::Uint8Array]
_get [HAPI_GetAttributeUInt8Data]
_get_async [HAPI_GetAttributeUInt8DataAsync]
_set [HAPI_SetAttributeUInt8Data]
_set_unique [HAPI_SetAttributeUInt8UniqueData]
_get_array [HAPI_GetAttributeUInt8ArrayData]
_set_array [HAPI_SetAttributeUInt8ArrayData]
]
[
_val_type [i8]
_storage [StorageType::Int8]
_storage_array [StorageType::Int8Array]
_get [HAPI_GetAttributeInt8Data]
_get_async [HAPI_GetAttributeInt8DataAsync]
_set [HAPI_SetAttributeInt8Data]
_set_unique [HAPI_SetAttributeInt8UniqueData]
_get_array [HAPI_GetAttributeInt8ArrayData]
_set_array [HAPI_SetAttributeInt8ArrayData]
]
[
_val_type [i16]
_storage [StorageType::Int16]
_storage_array [StorageType::Int16Array]
_get [HAPI_GetAttributeInt16Data]
_get_async [HAPI_GetAttributeInt16DataAsync]
_set [HAPI_SetAttributeInt16Data]
_set_unique [HAPI_SetAttributeInt16UniqueData]
_get_array [HAPI_GetAttributeInt16ArrayData]
_set_array [HAPI_SetAttributeInt16ArrayData]
]
[
_val_type [i32]
_storage [StorageType::Int]
_storage_array [StorageType::IntArray]
_get [HAPI_GetAttributeIntData]
_get_async [HAPI_GetAttributeIntDataAsync]
_set [HAPI_SetAttributeIntData]
_set_unique [HAPI_SetAttributeIntUniqueData]
_get_array [HAPI_GetAttributeIntArrayData]
_set_array [HAPI_SetAttributeIntArrayData]
]
[
_val_type [i64]
_storage [StorageType::Int64]
_storage_array [StorageType::Int64Array]
_get [HAPI_GetAttributeInt64Data]
_get_async [HAPI_GetAttributeInt64DataAsync]
_set [HAPI_SetAttributeInt64Data]
_set_unique [HAPI_SetAttributeInt64UniqueData]
_get_array [HAPI_GetAttributeInt64ArrayData]
_set_array [HAPI_SetAttributeInt64ArrayData]
]
[
_val_type [f32]
_storage [StorageType::Float]
_storage_array [StorageType::FloatArray]
_get [HAPI_GetAttributeFloatData]
_get_async [HAPI_GetAttributeFloatDataAsync]
_set [HAPI_SetAttributeFloatData]
_set_unique [HAPI_SetAttributeFloatUniqueData]
_get_array [HAPI_GetAttributeFloatArrayData]
_set_array [HAPI_SetAttributeFloatArrayData]
]
[
_val_type [f64]
_storage [StorageType::Float64]
_storage_array [StorageType::Float64Array]
_get [HAPI_GetAttributeFloat64Data]
_get_async [HAPI_GetAttributeFloat64DataAsync]
_set [HAPI_SetAttributeFloat64Data]
_set_unique [HAPI_SetAttributeFloat64UniqueData]
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
            (info.0.count * info.0.tupleSize) as usize,
            _val_type::default(),
        );
        unsafe {
            raw::_get(
                node.session.ptr(),
                node.handle.0,
                part,
                name.as_ptr(),
                info.ptr() as *mut _,
                -1,
                buffer.as_mut_ptr(),
                0,
                info.0.count,
            )
            .check_err(&node.session, || stringify!(Calling _get))
        }
    }
    fn get_async(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        buffer: &mut Vec<Self>,
    ) -> Result<i32> {
        debug_assert!(node.is_valid()?);
        // buffer capacity mut be of an appropriate size
        debug_assert!(buffer.capacity() >= (info.0.count * info.0.tupleSize) as usize);
        let mut job_id: i32 = -1;
        unsafe {
            raw::_get_async(
                node.session.ptr(),
                node.handle.0,
                part,
                name.as_ptr(),
                info.ptr() as *mut _,
                -1,
                buffer.as_mut_ptr(),
                0,
                info.0.count,
                &mut job_id as *mut _,
            )
            .check_err(&node.session, || stringify!(Calling _get_async))?;
            Ok(job_id)
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
                info.ptr(),
                data.as_ptr(),
                start,
                len,
            )
            .check_err(&node.session, || stringify!(Calling _set))
        }
    }

    fn set_unique(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part_id: i32,
        data: &[_val_type],
        start: i32,
    ) -> Result<()> {
        unsafe {
            raw::_set_unique(
                node.session.ptr(),
                node.handle.0,
                part_id,
                name.as_ptr(),
                info.ptr(),
                data.as_ptr(),
                info.0.tupleSize,
                start,
                info.0.count,
            )
            .check_err(&node.session, || stringify!(Calling _set_unique))
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
        let mut data = vec![_val_type::default(); info.0.totalArrayElements as usize];
        let mut sizes = vec![0; info.0.count as usize];
        unsafe {
            raw::_get_array(
                node.session.ptr(),
                node.handle.0,
                part,
                name.as_ptr(),
                &info.0 as *const _ as *mut _,
                data.as_mut_ptr(),
                info.0.totalArrayElements as i32,
                sizes.as_mut_ptr(),
                0,
                info.0.count,
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
                &info.0,
                data.as_ptr(),
                info.0.totalArrayElements as i32,
                sizes.as_ptr(),
                0,
                info.0.count,
            )
            .check_err(&node.session, || stringify!(Calling _set_array))?;
        }

        Ok(())
    }
}
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
    attr_info: &HAPI_AttributeInfo,
) -> Result<StringArray> {
    debug_assert!(node.is_valid()?);
    debug_assert!(attr_info.count > 0);
    unsafe {
        let mut handles = vec![StringHandle(0); attr_info.count as usize];
        // SAFETY: Most likely an error in C API, it should not modify the info object,
        // but for some reason it wants a mut pointer
        let attr_info = attr_info as *const _ as *mut HAPI_AttributeInfo;
        raw::_get_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            handles.as_mut_ptr() as *mut HAPI_StringHandle,
            0,
            handles.len() as i32,
        )
        .check_err(&node.session, || stringify!(Calling _get_ffi_fn))?;
        crate::stringhandle::get_string_array(&handles, &node.session)
    }
}

#[derive(Debug)]
// TODO: Think of better name?
pub struct AsyncResult<T: Sized + Send + 'static> {
    pub(crate) job_id: i32,
    pub(crate) data: T,
    pub(crate) session: Session,
}

impl<T: Sized + Send + 'static> AsyncResult<T> {
    pub fn is_ready(&self) -> Result<bool> {
        self.session
            .get_job_status(self.job_id)
            .map(|status| status == crate::session::JobStatus::Idle)
    }

    pub fn wait(self) -> Result<T> {
        loop {
            if self.is_ready()? {
                return Ok(self.data);
            }
        }
    }
}

// Async versions for string and dict
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
    attr_info: &HAPI_AttributeInfo,
) -> Result<AsyncResult<StringArray>> {
    unsafe {
        let mut handles = vec![StringHandle(0); attr_info.count as usize];
        let session = node.session.clone();
        // SAFETY: Most likely an error in C API, it should not modify the info object,
        // but for some reason it wants a mut pointer
        let attr_info = attr_info as *const _ as *mut HAPI_AttributeInfo;
        let mut job_id = -1;
        raw::_get_async_ffi_fn(
            session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            handles.as_mut_ptr() as *mut HAPI_StringHandle,
            0,
            handles.len() as i32,
            &mut job_id,
        )
        .check_err(&session, || stringify!(Calling _get_async_ffi_fn))?;
        let data = crate::stringhandle::get_string_array(&handles, &session)?;
        Ok(AsyncResult {
            job_id,
            data,
            session,
        })
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
pub(crate) fn _rust_fn(
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
            attr_info as *const _,
            array.as_mut_ptr(),
            0,
            array.len() as i32,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))
    }
}

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
            data_array.as_mut_ptr() as *mut HAPI_StringHandle,
            info.total_array_elements() as i32,
            sizes_fixed_array.as_mut_ptr(),
            0,
            sizes_fixed_array.len() as i32,
        )
        .check_err(&node.session, || stringify!(Calling _ffi_fn))?;

        Ok(StringMultiArray {
            handles: data_array,
            sizes: sizes_fixed_array,
            session: node.session.clone(),
        })
    }
}

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
    info: &HAPI_AttributeInfo,
    data: &mut [*const i8],
    sizes: &[i32],
) -> Result<()> {
    debug_assert!(node.is_valid()?);
    unsafe {
        raw::_ffi_fn(
            node.session.ptr(),
            node.handle.0,
            0,
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

#[duplicate_item(
[
_rust_fn [set_attribute_string_unique_data]
_ffi_fn [HAPI_SetAttributeStringUniqueData]
_val_type [*const ::std::os::raw::c_char]
]

)]

pub(crate) unsafe fn _rust_fn(
    node: &HoudiniNode,
    name: &CStr,
    info: &HAPI_AttributeInfo,
    part: i32,
    data: _val_type,
) -> Result<()> {
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
