use crate::attribute::{array::DataArray, AttribAccess, JobId};
use crate::ffi::raw;
use crate::ffi::raw::StorageType;
use crate::ffi::AttributeInfo;
use crate::{node::HoudiniNode, Result};
use duplicate::duplicate_item;
use std::ffi::CStr;

#[duplicate_item(
[
_val_type [u8]
_storage [StorageType::Uint8]
_storage_array [StorageType::Uint8Array]
_get [HAPI_GetAttributeUInt8Data]
_get_async [HAPI_GetAttributeUInt8DataAsync]
_get_array [HAPI_GetAttributeUInt8ArrayData]
_get_array_async [HAPI_GetAttributeUInt8ArrayDataAsync]
_set [HAPI_SetAttributeUInt8Data]
_set_async [HAPI_SetAttributeUInt8DataAsync]
_set_unique [HAPI_SetAttributeUInt8UniqueData]
_set_unique_async [HAPI_SetAttributeUInt8UniqueDataAsync]
_set_array [HAPI_SetAttributeUInt8ArrayData]
_set_array_async [HAPI_SetAttributeUInt8ArrayDataAsync]
]
[
_val_type [i8]
_storage [StorageType::Int8]
_storage_array [StorageType::Int8Array]
_get [HAPI_GetAttributeInt8Data]
_get_async [HAPI_GetAttributeInt8DataAsync]
_get_array [HAPI_GetAttributeInt8ArrayData]
_get_array_async [HAPI_GetAttributeInt8ArrayDataAsync]
_set [HAPI_SetAttributeInt8Data]
_set_async [HAPI_SetAttributeInt8DataAsync]
_set_unique [HAPI_SetAttributeInt8UniqueData]
_set_unique_async [HAPI_SetAttributeInt8UniqueDataAsync]
_set_array [HAPI_SetAttributeInt8ArrayData]
_set_array_async [HAPI_SetAttributeInt8ArrayDataAsync]
]
[
_val_type [i16]
_storage [StorageType::Int16]
_storage_array [StorageType::Int16Array]
_get [HAPI_GetAttributeInt16Data]
_get_async [HAPI_GetAttributeInt16DataAsync]
_get_array [HAPI_GetAttributeInt16ArrayData]
_get_array_async [HAPI_GetAttributeInt16ArrayDataAsync]

_set [HAPI_SetAttributeInt16Data]
_set_async [HAPI_SetAttributeInt16DataAsync]
_set_unique [HAPI_SetAttributeInt16UniqueData]
_set_unique_async [HAPI_SetAttributeInt16UniqueDataAsync]
_set_array [HAPI_SetAttributeInt16ArrayData]
_set_array_async [HAPI_SetAttributeInt16ArrayDataAsync]
]
[
_val_type [i32]
_storage [StorageType::Int]
_storage_array [StorageType::IntArray]
_get [HAPI_GetAttributeIntData]
_get_async [HAPI_GetAttributeIntDataAsync]
_get_array [HAPI_GetAttributeIntArrayData]
_get_array_async [HAPI_GetAttributeIntArrayDataAsync]

_set [HAPI_SetAttributeIntData]
_set_async [HAPI_SetAttributeIntDataAsync]
_set_unique [HAPI_SetAttributeIntUniqueData]
_set_unique_async [HAPI_SetAttributeIntUniqueDataAsync]
_set_array [HAPI_SetAttributeIntArrayData]
_set_array_async [HAPI_SetAttributeIntArrayDataAsync]
]
[
_val_type [i64]
_storage [StorageType::Int64]
_storage_array [StorageType::Int64Array]
_get [HAPI_GetAttributeInt64Data]
_get_async [HAPI_GetAttributeInt64DataAsync]
_get_array [HAPI_GetAttributeInt64ArrayData]
_get_array_async [HAPI_GetAttributeInt64ArrayDataAsync]
_set [HAPI_SetAttributeInt64Data]
_set_async [HAPI_SetAttributeInt64DataAsync]
_set_unique [HAPI_SetAttributeInt64UniqueData]
_set_unique_async [HAPI_SetAttributeInt64UniqueDataAsync]
_set_array [HAPI_SetAttributeInt64ArrayData]
_set_array_async [HAPI_SetAttributeInt64ArrayDataAsync]
]
[
_val_type [f32]
_storage [StorageType::Float]
_storage_array [StorageType::FloatArray]
_get [HAPI_GetAttributeFloatData]
_get_async [HAPI_GetAttributeFloatDataAsync]
_get_array [HAPI_GetAttributeFloatArrayData]
_get_array_async [HAPI_GetAttributeFloatArrayDataAsync]
_set [HAPI_SetAttributeFloatData]
_set_async [HAPI_SetAttributeFloatDataAsync]
_set_unique [HAPI_SetAttributeFloatUniqueData]
_set_unique_async [HAPI_SetAttributeFloatUniqueDataAsync]
_set_array [HAPI_SetAttributeFloatArrayData]
_set_array_async [HAPI_SetAttributeFloatArrayDataAsync]
]
[
_val_type [f64]
_storage [StorageType::Float64]
_storage_array [StorageType::Float64Array]
_get [HAPI_GetAttributeFloat64Data]
_get_async [HAPI_GetAttributeFloat64DataAsync]
_get_array [HAPI_GetAttributeFloat64ArrayData]
_get_array_async [HAPI_GetAttributeFloat64ArrayDataAsync]
_set [HAPI_SetAttributeFloat64Data]
_set_async [HAPI_SetAttributeFloat64DataAsync]
_set_unique [HAPI_SetAttributeFloat64UniqueData]
_set_unique_async [HAPI_SetAttributeFloat64UniqueDataAsync]
_set_array [HAPI_SetAttributeFloat64ArrayData]
_set_array_async [HAPI_SetAttributeFloat64ArrayDataAsync]
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
    ) -> Result<JobId> {
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

    fn set_async(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        data: &[_val_type],
        start: i32,
        len: i32,
    ) -> Result<JobId> {
        debug_assert!(node.is_valid()?);
        let mut job_id: i32 = -1;
        unsafe {
            raw::_set_async(
                node.session.ptr(),
                node.handle.0,
                part,
                name.as_ptr(),
                info.ptr(),
                data.as_ptr(),
                start,
                len,
                &mut job_id as *mut _,
            )
            .check_err(&node.session, || stringify!(Calling _set_async))?;
        }
        Ok(job_id)
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

    fn set_unique_async(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part_id: i32,
        data: &[_val_type],
        start: i32,
    ) -> Result<JobId> {
        let mut job_id: i32 = -1;
        unsafe {
            raw::_set_unique_async(
                node.session.ptr(),
                node.handle.0,
                part_id,
                name.as_ptr(),
                info.ptr(),
                data.as_ptr(),
                info.0.tupleSize,
                start,
                info.0.count,
                &mut job_id as *mut _,
            )
            .check_err(&node.session, || stringify!(Calling _set_unique_async))?;
        }

        Ok(job_id)
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

    fn get_array_async(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        data: &mut [Self],
        sizes: &mut [i32],
        part: i32,
    ) -> Result<JobId> {
        let mut job_id: i32 = -1;
        unsafe {
            raw::_get_array_async(
                node.session.ptr(),                 // 	const HAPI_Session * 	session,
                node.handle.0,                      // HAPI_NodeId 	node_id,
                part,                               // HAPI_PartId 	part_id,
                name.as_ptr(),                      // const char * 	attr_name,
                info.ptr() as *mut _,               // HAPI_AttributeInfo * 	attr_info,
                data.as_mut_ptr(),                  // HAPI_UInt8 * 	data_fixed_array,
                info.total_array_elements() as i32, // int 	data_fixed_length,
                sizes.as_mut_ptr(),                 // int * 	sizes_fixed_array,
                0,                                  // int 	start,
                info.count(),                       // int 	sizes_fixed_length,
                &mut job_id as *mut _,              // int * 	job_id
            )
            .check_err(&node.session, || stringify!(Calling _get_array_async))?;
        }
        Ok(job_id)
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

    fn set_array_async(
        name: &CStr,
        node: &HoudiniNode,
        info: &AttributeInfo,
        part: i32,
        data: &[_val_type],
        sizes: &[i32],
    ) -> Result<JobId> {
        let mut job_id: i32 = -1;
        unsafe {
            raw::_set_array_async(
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
                &mut job_id as *mut _,
            )
            .check_err(&node.session, || stringify!(Calling set_array_async))?;
        }

        Ok(job_id)
    }
}
