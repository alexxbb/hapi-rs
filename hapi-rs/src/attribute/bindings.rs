use super::array::DataArray;
use crate::ffi::raw;
use crate::stringhandle::StringArray;
use crate::{node::HoudiniNode, Result};
use duplicate::duplicate;
use std::ffi::CStr;

#[duplicate(
_data_type _func_name _default _ffi_func;
[u8] [get_attribute_u8_data] [0] [HAPI_GetAttributeUInt8Data];
[i8] [get_attribute_i8_data] [0] [HAPI_GetAttributeInt8Data];
[i16] [get_attribute_i16_data] [0] [HAPI_GetAttributeInt16Data];
[i32] [get_attribute_i32_data] [0] [HAPI_GetAttributeIntData];
[i64] [get_attribute_i64_data] [0] [HAPI_GetAttributeInt64Data];
[f32] [get_attribute_f32_data] [0.0] [HAPI_GetAttributeFloatData];
[f64] [get_attribute_f64_data] [0.0] [HAPI_GetAttributeFloat64Data];
)]
pub fn _func_name(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &raw::HAPI_AttributeInfo,
    stride: i32,
    start: i32,
    length: i32,
) -> Result<Vec<_data_type>> {
    unsafe {
        let mut data_array = Vec::new();
        data_array.resize((length * attr_info.tupleSize) as usize, _default);
        // SAFETY: Most likely an error in C API, it should not modify the info object,
        // but for some reason it wants a mut pointer
        let attr_info = attr_info as *const _ as *mut raw::HAPI_AttributeInfo;
        // let mut data_array = vec![];
        raw::_ffi_func(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            stride,
            data_array.as_mut_ptr(),
            start,
            length,
        )
        .check_err(Some(&node.session))?;
        Ok(data_array)
    }
}

#[duplicate(
_data_type _func_name _ffi_name;
[i8] [get_attribute_i8_array_data] [HAPI_GetAttributeInt8ArrayData];
[u8] [get_attribute_u8_array_data] [HAPI_GetAttributeUInt8ArrayData];
[i16] [get_attribute_i16_array_data] [HAPI_GetAttributeInt16ArrayData];
[i32] [get_attribute_int_array_data] [HAPI_GetAttributeIntArrayData];
[i64] [get_attribute_int64_array_data] [HAPI_GetAttributeInt64ArrayData];
[f32] [get_attribute_float_array_data] [HAPI_GetAttributeFloatArrayData];
[f64] [get_attribute_float64_array_data] [HAPI_GetAttributeFloat64ArrayData];
)]
pub fn _func_name(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &raw::HAPI_AttributeInfo,
) -> Result<DataArray<'static, _data_type>> {
    let mut data = vec![_data_type::default(); attr_info.totalArrayElements as usize];
    let mut sizes = vec![0; attr_info.count as usize];
    unsafe {
        raw::_ffi_name(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info as *const _ as *mut _,
            data.as_mut_ptr(),
            attr_info.totalArrayElements as i32,
            sizes.as_mut_ptr(),
            0,
            attr_info.count as i32,
        )
        .check_err(Some(&node.session))?;
    }

    Ok(DataArray::new_owned(data, sizes))
}

#[duplicate(
_data_type _func_name _ffi_func;
[i8] [set_attribute_i8_data] [HAPI_SetAttributeInt8Data];
[u8] [set_attribute_u8_data] [HAPI_SetAttributeUInt8Data];
[i16] [set_attribute_i16_data] [HAPI_SetAttributeInt16Data];
[i32] [set_attribute_int_data] [HAPI_SetAttributeIntData];
[i64] [set_attribute_int64_data] [HAPI_SetAttributeInt64Data];
[f32] [set_attribute_float_data]  [HAPI_SetAttributeFloatData];
[f64] [set_attribute_float64_data]  [HAPI_SetAttributeFloat64Data];
)]
pub fn _func_name(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &raw::HAPI_AttributeInfo,
    data_array: &[_data_type],
    start: i32,
    length: i32,
) -> Result<()> {
    unsafe {
        raw::_ffi_func(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
            data_array.as_ptr(),
            start,
            length,
        )
        .check_err(Some(&node.session))
    }
}

#[duplicate(
_data_type _func_name _ffi_name;
[u8] [set_attribute_u8_array_data] [HAPI_SetAttributeUInt8ArrayData];
[i8] [set_attribute_i8_array_data] [HAPI_SetAttributeInt8ArrayData];
[i16] [set_attribute_i16_array_data] [HAPI_SetAttributeInt16ArrayData];
[i32] [set_attribute_i32_array_data] [HAPI_SetAttributeIntArrayData];
[i64] [set_attribute_i64_array_data] [HAPI_SetAttributeInt64ArrayData];
[f32] [set_attribute_f32_array_data] [HAPI_SetAttributeFloatArrayData];
[f64] [set_attribute_f64_array_data] [HAPI_SetAttributeFloat64ArrayData];
)]
pub fn _func_name(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &raw::HAPI_AttributeInfo,
    data: &[_data_type],
    sizes: &[i32],
) -> Result<()> {
    unsafe {
        raw::_ffi_name(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info as *const _,
            data.as_ptr(),
            attr_info.totalArrayElements as i32,
            sizes.as_ptr(),
            0,
            attr_info.count as i32,
        )
        .check_err(Some(&node.session))?;
    }

    Ok(())
}

pub fn get_attribute_string_data(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &raw::HAPI_AttributeInfo,
) -> Result<StringArray> {
    unsafe {
        let mut handles = Vec::new();
        let count = attr_info.count;
        handles.resize((count * attr_info.tupleSize) as usize, 0);
        // SAFETY: Most likely an error in C API, it should not modify the info object,
        // but for some reason it wants a mut pointer
        let attr_info = attr_info as *const _ as *mut raw::HAPI_AttributeInfo;
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
    attr_info: &raw::HAPI_AttributeInfo,
    array: &[&CStr],
) -> Result<()> {
    unsafe {
        // SAFETY: Most likely an error in C API, it should not modify the info object,
        // but for some reason it wants a mut pointer
        // TODO: Is there a way to set Vec capacity with collect?
        // let mut array =  Vec::with_capacity(array.len());
        let mut array = Vec::from_iter(array.iter().map(|cs| cs.as_ptr()));
        // let mut array = array.iter().map(|cs|cs.as_ptr()).collect::<Vec<_>>();
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
