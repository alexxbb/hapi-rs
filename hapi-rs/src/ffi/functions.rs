use crate::{errors::Result, node::{HoudiniNode, NodeHandle}, parameter::ParmHandle, session::Session};

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

pub fn is_node_valid(info: &super::NodeInfo) -> Result<bool> {
    unsafe {
        let mut answer = MaybeUninit::uninit();
        raw::HAPI_IsNodeValid(info.session.ptr(), info.inner.id,
                              info.inner.uniqueHoudiniNodeId,
                              answer.as_mut_ptr())
            .result_with_session(|| info.session.clone())?;
        Ok(answer.assume_init() == 1)
    }
}

pub fn delete_node(node: HoudiniNode) -> Result<()> {
    unsafe {
        raw::HAPI_DeleteNode(node.session.ptr(), node.handle.0)
            .result_with_session(|| node.session.clone())
    }
}

pub fn get_node_path(node: &HoudiniNode, relative_to: Option<&HoudiniNode>) -> Result<String> {
    unsafe {
        let mut sh = MaybeUninit::uninit();
        raw::HAPI_GetNodePath(
            node.session.ptr(),
            node.handle.0,
            relative_to.map(|n| n.handle.0).unwrap_or(-1),
            sh.as_mut_ptr(),
        )
            .result_with_session(|| node.session.clone())?;
        crate::stringhandle::get_string(sh.assume_init(), &node.session)
    }
}

pub fn cook_node(node: &HoudiniNode, options: *const raw::HAPI_CookOptions) -> Result<()> {
    unsafe {
        raw::HAPI_CookNode(node.session.ptr(), node.handle.0, options)
            .result_with_session(|| node.session.clone())
    }
}

pub fn load_library_from_file(path: &CStr, session: &Session, _override: bool) -> Result<i32> {
    unsafe {
        let mut lib_id = MaybeUninit::uninit();
        raw::HAPI_LoadAssetLibraryFromFile(
            session.ptr(),
            path.as_ptr(),
            _override as i8,
            lib_id.as_mut_ptr(),
        )
            .result_with_session(|| session.clone())?;
        Ok(lib_id.assume_init())
    }
}

pub fn get_asset_info(node: &HoudiniNode) -> Result<raw::HAPI_AssetInfo> {
    unsafe {
        let mut info = MaybeUninit::uninit();
        raw::HAPI_GetAssetInfo(node.session.ptr(), node.handle.0, info.as_mut_ptr())
            .result_with_session(|| node.session.clone())?;
        Ok(info.assume_init())
    }
}

pub fn get_asset_count(library_id: i32, session: &Session) -> Result<i32> {
    unsafe {
        let mut num_assets = MaybeUninit::uninit();
        raw::HAPI_GetAvailableAssetCount(
            session.ptr(),
            library_id,
            num_assets.as_mut_ptr(),
        ).result_with_session(|| session.clone())?;
        Ok(num_assets.assume_init())
    }
}

pub fn get_asset_names(library_id: i32, num_assets: i32, session: &Session) -> Result<Vec<String>> {
    let handles = unsafe {
        let mut names = vec![0; num_assets as usize];
        raw::HAPI_GetAvailableAssets(
            session.ptr(),
            library_id,
            names.as_mut_ptr(),
            num_assets,
        ).result_with_session(|| session.clone())?;
        names
    };
    crate::stringhandle::get_string_batch(&handles, session)
}

pub fn get_asset_def_parm_count(library_id: i32, session: &Session) -> Result<()> {
    unimplemented!()
    // unsafe {
    //     raw::HAPI_GetAssetDefinitionParmCounts(
    //         session.ptr(),
    //         library_id,
    //         asset_name.as_ptr(),
    //         num_parms.as_mut_ptr(),
    //         a1.as_mut_ptr(),
    //         a2.as_mut_ptr(),
    //         a3.as_mut_ptr(),
    //         a4.as_mut_ptr(),
    //     ).result_with_session(|| session.clone())?;
    // }
    // Ok(())
}

pub fn get_asset_parm_info() -> Result<()> {
    unimplemented!()
    // ffi::HAPI_GetAssetDefinitionParmInfos(
    //     self.session.ptr(),
    //     self.lib_id,
    //     asset_name.as_ptr(),
    //     parms.as_mut_ptr(),
    //     0,
    //     num_parms,
    // ).result_with_session(|| self.session.clone())?;
}

pub fn get_string_batch_size(handles: &[i32], session: &Session) -> Result<i32> {
    unsafe {
        let mut length = MaybeUninit::uninit();
        raw::HAPI_GetStringBatchSize(
            session.ptr(),
            handles.as_ptr(),
            handles.len() as i32,
            length.as_mut_ptr(),
        )
            .result_with_session(|| session.clone())?;
        Ok(length.assume_init())
    }
}

pub fn get_string_batch(length: i32, session: &Session) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; length as usize];
    unsafe {
        raw::HAPI_GetStringBatch(session.ptr(),
                                 buffer.as_mut_ptr() as *mut _,
                                 length as i32)
            .result_with_session(|| session.clone())?;
    }
    buffer.truncate(length as usize - 1);
    Ok(buffer)
}

pub fn get_string_buff_len(session: &Session, handle: i32) -> Result<i32> {
    unsafe {
        let mut length = MaybeUninit::uninit();
        raw::HAPI_GetStringBufLength(session.ptr(), handle, length.as_mut_ptr())
            .result_with_session(|| session.clone())?;
        Ok(length.assume_init())
    }
}

pub fn get_string(session: &Session, handle: i32, length: i32) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; length as usize];
    unsafe {
        raw::HAPI_GetString(session.ptr(),
                            handle,
                            buffer.as_mut_ptr() as *mut _,
                            length)
            .result_with_message("get_string failed")?;
        buffer.truncate(length as usize - 1);
    }
    Ok(buffer)
}

pub fn create_inprocess_session() -> Result<raw::HAPI_Session> {
    let mut ses = MaybeUninit::uninit();
    unsafe {
        raw::HAPI_CreateInProcessSession(ses.as_mut_ptr())
            .result_with_message("Session::new_in_process failed")?;
        Ok(ses.assume_init())
    }
}