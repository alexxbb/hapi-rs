use crate::{errors::Result, ffi, node::NodeHandle, parameter::ParmHandle, session::Session};
use std::ffi::CStr;
use std::mem::MaybeUninit;

pub(crate) fn get_float_values(
    node: &NodeHandle,
    session: &Session,
    start: i32,
    count: i32,
) -> Result<Vec<f32>> {
    let mut values = vec![0.; count as usize];
    unsafe {
        ffi::HAPI_GetParmFloatValues(session.ptr(), node.0, values.as_mut_ptr(), start, count)
            .result_with_session(|| session.clone())?
    }
    Ok(values)
}

pub(crate) fn get_int_values(
    node: &NodeHandle,
    session: &Session,
    start: i32,
    length: i32,
) -> Result<Vec<i32>> {
    let mut values = vec![0; length as usize];
    unsafe {
        ffi::HAPI_GetParmIntValues(
            session.ptr(),
            node.0,
            values.as_mut_ptr(),
            start,
            length)
            .result_with_session(|| session.clone())?
    }
    Ok(values)
}

pub(crate) fn get_string_values(
    node: &NodeHandle,
    session: &Session,
    start: i32,
    length: i32,
) -> Result<Vec<String>> {
    let mut handles = vec![0; length as usize];
    unsafe {
        ffi::HAPI_GetParmStringValues(
            session.ptr(),
            node.0,
            1,
            handles.as_mut_ptr(),
            start,
            length)
            .result_with_session(|| session.clone())?
    }
    crate::stringhandle::get_string_batch(&handles, &session)
}

pub(crate) fn get_float_value(
    node: &NodeHandle,
    session: &Session,
    name: &CStr,
    index: i32,
) -> Result<f32> {
    let mut value = MaybeUninit::uninit();

    unsafe {
        ffi::HAPI_GetParmFloatValue(
            session.ptr(),
            node.0,
            name.as_ptr(),
            index,
            value.as_mut_ptr(),
        )
            .result_with_session(|| session.clone());
        Ok(value.assume_init())
    }
}

pub(crate) fn get_int_value(
    node: &NodeHandle,
    session: &Session,
    name: &CStr,
    index: i32,
) -> Result<i32> {
    let mut value = MaybeUninit::uninit();

    unsafe {
        ffi::HAPI_GetParmIntValue(
            session.ptr(),
            node.0,
            name.as_ptr(),
            index,
            value.as_mut_ptr(),
        )
            .result_with_session(|| session.clone());
        Ok(value.assume_init())
    }
}

pub(crate) fn get_string_value(
    node: &NodeHandle,
    session: &Session,
    name: &CStr,
    index: i32,
) -> Result<String> {
    let mut handle = MaybeUninit::uninit();
    let handle = unsafe {
        ffi::HAPI_GetParmStringValue(
            session.ptr(),
            node.0,
            name.as_ptr(),
            index,
            1,
            handle.as_mut_ptr(),
        )
            .result_with_session(|| session.clone());
        handle.assume_init()
    };
    session.get_string(handle)
}

pub(crate) fn set_float_value(
    node: &NodeHandle,
    session: &Session,
    name: &CStr,
    index: i32,
    value: f32,
) -> Result<()> {
    unsafe {
        ffi::HAPI_SetParmFloatValue(
            session.ptr(),
            node.0,
            name.as_ptr(),
            index,
            value,
        ).result_with_session(|| session.clone())
    }
}

pub(crate) fn set_float_values(
    node: &NodeHandle,
    session: &Session,
    start: i32,
    length: i32,
    values: &[f32],
) -> Result<()> {
    unsafe {
        ffi::HAPI_SetParmFloatValues(
            session.ptr(),
            node.0,
            values.as_ptr(),
            start,
            length,
        ).result_with_session(|| session.clone())
    }
}

pub(crate) fn set_int_values(
    node: &NodeHandle,
    session: &Session,
    start: i32,
    length: i32,
    values: &[i32],
) -> Result<()> {
    unsafe {
        ffi::HAPI_SetParmIntValues(
            session.ptr(),
            node.0,
            values.as_ptr(),
            start,
            length,
        ).result_with_session(|| session.clone())
    }
}

pub(crate) fn set_int_value(
    node: &NodeHandle,
    session: &Session,
    name: &CStr,
    index: i32,
    value: i32,
) -> Result<()> {
    unsafe {
        ffi::HAPI_SetParmIntValue(
            session.ptr(),
            node.0,
            name.as_ptr(),
            index,
            value,
        ).result_with_session(|| session.clone())
    }
}

pub(crate) fn set_string_value(
    node: &NodeHandle,
    parm: &ParmHandle,
    session: &Session,
    index: i32,
    value: &CStr,
) -> Result<()> {
    unsafe {
        ffi::HAPI_SetParmStringValue(
            session.ptr(),
            node.0,
            value.as_ptr(),
            parm.0,
            index,
        ).result_with_session(|| session.clone())
    }
}

pub(crate) fn set_string_values(
    node: &NodeHandle,
    parm: &ParmHandle,
    session: &Session,
    values: &[&CStr],
) -> Result<()> {
    for (i, v) in values.iter().enumerate() {
        set_string_value(node, parm, session, i as i32, v)?;
    }
    Ok(())
}
