use crate::{errors::Result, node::{HoudiniNode,NodeHandle}, parameter::ParmHandle, session::Session};

use std::ffi::CStr;
use std::mem::MaybeUninit;

use super::raw;

pub fn get_parm_float_values(
    node: &HoudiniNode,
    start: i32,
    count: i32,
) -> Result<Vec<f32>> {
    let mut values = vec![0.; count as usize];
    unsafe {
        raw::HAPI_GetParmFloatValues(node.session.ptr(), node.handle.0, values.as_mut_ptr(), start, count)
            .result_with_session(|| node.session.clone())?
    }
    Ok(values)
}

pub fn get_parm_int_values(
    node: &HoudiniNode,
    start: i32,
    length: i32,
) -> Result<Vec<i32>> {
    let mut values = vec![0; length as usize];
    unsafe {
        raw::HAPI_GetParmIntValues(
            node.session.ptr(),
            node.handle.0,
            values.as_mut_ptr(),
            start,
            length)
            .result_with_session(|| node.session.clone())?
    }
    Ok(values)
}

pub fn get_parm_string_values(
    node: &HoudiniNode,
    start: i32,
    length: i32,
) -> Result<Vec<String>> {
    let mut handles = vec![0; length as usize];
    unsafe {
        raw::HAPI_GetParmStringValues(
            node.session.ptr(),
            node.handle.0,
            1,
            handles.as_mut_ptr(),
            start,
            length)
            .result_with_session(|| node.session.clone())?
    }
    crate::stringhandle::get_string_batch(&handles, &node.session)
}

pub fn get_parm_float_value(
    node: &HoudiniNode,
    name: &CStr,
    index: i32,
) -> Result<f32> {
    let mut value = MaybeUninit::uninit();

    unsafe {
        raw::HAPI_GetParmFloatValue(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            index,
            value.as_mut_ptr(),
        )
            .result_with_session(|| node.session.clone());
        Ok(value.assume_init())
    }
}

pub fn get_parm_int_value(
    node: &HoudiniNode,
    name: &CStr,
    index: i32,
) -> Result<i32> {
    let mut value = MaybeUninit::uninit();

    unsafe {
        raw::HAPI_GetParmIntValue(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            index,
            value.as_mut_ptr(),
        )
            .result_with_session(|| node.session.clone());
        Ok(value.assume_init())
    }
}

pub fn get_parm_string_value(
    node: &HoudiniNode,
    name: &CStr,
    index: i32,
) -> Result<String> {
    let mut handle = MaybeUninit::uninit();
    let handle = unsafe {
        raw::HAPI_GetParmStringValue(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            index,
            1,
            handle.as_mut_ptr(),
        )
            .result_with_session(|| node.session.clone());
        handle.assume_init()
    };
    node.session.get_string(handle)
}

pub fn set_parm_float_value(
    node: &HoudiniNode,
    name: &CStr,
    index: i32,
    value: f32,
) -> Result<()> {
    unsafe {
        raw::HAPI_SetParmFloatValue(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            index,
            value,
        ).result_with_session(|| node.session.clone())
    }
}

pub fn set_parm_float_values(
    node: &HoudiniNode,
    start: i32,
    length: i32,
    values: &[f32],
) -> Result<()> {
    if values.len() as i32 > length {
        log::warn!("Array length is greater than parm length: {:?}", values);
    }
    let length = values.len().min(length as usize);
    unsafe {
        raw::HAPI_SetParmFloatValues(
            node.session.ptr(),
            node.handle.0,
            values.as_ptr(),
            start,
            length as i32,
        ).result_with_session(|| node.session.clone())
    }
}

pub fn set_parm_int_values(
    node: &HoudiniNode,
    start: i32,
    length: i32,
    values: &[i32],
) -> Result<()> {
    unsafe {
        raw::HAPI_SetParmIntValues(
            node.session.ptr(),
            node.handle.0,
            values.as_ptr(),
            start,
            length,
        ).result_with_session(|| node.session.clone())
    }
}

pub fn set_parm_int_value(
    node: &HoudiniNode,
    name: &CStr,
    index: i32,
    value: i32,
) -> Result<()> {
    unsafe {
        raw::HAPI_SetParmIntValue(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            index,
            value,
        ).result_with_session(|| node.session.clone())
    }
}

pub fn set_parm_string_value(
    node: &HoudiniNode,
    parm: &ParmHandle,
    index: i32,
    value: &CStr,
) -> Result<()> {
    unsafe {
        raw::HAPI_SetParmStringValue(
            node.session.ptr(),
            node.handle.0,
            value.as_ptr(),
            parm.0,
            index,
        ).result_with_session(|| node.session.clone())
    }
}

pub fn set_parm_string_values<T>(
    node: &HoudiniNode,
    parm: &ParmHandle,
    values: &[T],
) -> Result<()>
    where T: AsRef<CStr>
{
    for (i, v) in values.iter().enumerate() {
        set_parm_string_value(node, parm, i as i32, v.as_ref())?;
    }
    Ok(())
}

pub fn get_parm_choice_list(
    node: &HoudiniNode,
    index: i32,
    length: i32,
) -> Result<Vec<raw::HAPI_ParmChoiceInfo>> {
    unsafe {
        let mut structs = vec![raw::HAPI_ParmChoiceInfo_Create(); length as usize];
        raw::HAPI_GetParmChoiceLists(node.session.ptr(), node.handle.0, structs.as_mut_ptr(), index, length)
            .result_with_session(|| node.session.clone())?;
        Ok(structs)
    }
}

pub fn get_parm_expression(
    node: &HoudiniNode,
    parm: &CStr,
    index: i32,
) -> Result<String> {
    let handle = unsafe {
        let mut handle = MaybeUninit::uninit();
        raw::HAPI_GetParmExpression(node.session.ptr(), node.handle.0, parm.as_ptr(), index, handle.as_mut_ptr())
            .result_with_session(|| node.session.clone())?;
        handle.assume_init()
    };
    crate::stringhandle::get_string(handle, &node.session)
}

pub fn set_parm_expression(
    node: &HoudiniNode,
    parm: &ParmHandle,
    value: &CStr,
    index: i32,
) -> Result<()> {
    unsafe {
        raw::HAPI_SetParmExpression(node.session.ptr(),
                                    node.handle.0,
                                    value.as_ptr(),
                                    parm.0,
                                    index)
            .result_with_session(|| node.session.clone())
    }
}

pub fn get_parm_info(node: &HoudiniNode, parm: &ParmHandle) -> Result<raw::HAPI_ParmInfo> {
    unsafe {
        let mut info = MaybeUninit::uninit();
        super::raw::HAPI_GetParmInfo(
            node.session.ptr(),
            node.handle.0,
            parm.0,
            info.as_mut_ptr(),
        ).result_with_session(|| node.session.clone())?;
        Ok(info.assume_init())
    }
}

pub fn get_parm_info_from_name(node: &HoudiniNode, name: &CStr) -> Result<raw::HAPI_ParmInfo> {
    unsafe {
        let mut info = MaybeUninit::uninit();
        super::raw::HAPI_GetParmInfoFromName(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            info.as_mut_ptr(),
        ).result_with_session(|| node.session.clone())?;
        Ok(info.assume_init())
    }
}

pub fn get_parm_id_from_name(name: &CStr, node: &HoudiniNode) -> Result<i32> {
    unsafe {
        let mut id = MaybeUninit::uninit();
        crate::ffi::raw::HAPI_GetParmIdFromName(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            id.as_mut_ptr(),
        ).result_with_session(|| node.session.clone())?;
        Ok(id.assume_init())
    }
}

pub fn get_node_info(node: &NodeHandle, session: &Session) -> Result<raw::HAPI_NodeInfo> {
    unsafe {
        let mut info = MaybeUninit::uninit();
        super::raw::HAPI_GetNodeInfo(
            session.ptr(),
            node.0,
            info.as_mut_ptr(),
        ).result_with_session(|| session.clone())?;
        Ok(info.assume_init())
    }
}
