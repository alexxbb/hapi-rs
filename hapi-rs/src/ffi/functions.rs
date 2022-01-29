#![allow(clippy::missing_safety_doc)]

use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};

use crate::ffi::{CookOptions, CurveInfo, GeoInfo, ImageInfo, PartInfo, Viewport};
use crate::{
    errors::{HapiError, Kind, Result},
    node::{HoudiniNode, NodeHandle},
    parameter::ParmHandle,
    session::{Session, SessionOptions},
    stringhandle::StringArray,
};

use super::raw;

macro_rules! uninit {
    () => {
        MaybeUninit::uninit()
    };
}

pub fn get_parm_float_values(
    node: NodeHandle,
    session: &Session,
    start: i32,
    count: i32,
) -> Result<Vec<f32>> {
    let mut values = vec![0.; count as usize];
    unsafe {
        raw::HAPI_GetParmFloatValues(session.ptr(), node.0, values.as_mut_ptr(), start, count)
            .check_err(Some(session))?
    }
    Ok(values)
}

pub fn get_parm_int_values(
    node: NodeHandle,
    session: &Session,
    start: i32,
    length: i32,
) -> Result<Vec<i32>> {
    let mut values = vec![0; length as usize];
    unsafe {
        raw::HAPI_GetParmIntValues(session.ptr(), node.0, values.as_mut_ptr(), start, length)
            .check_err(Some(session))?
    }
    Ok(values)
}

pub fn get_parm_string_values(
    node: NodeHandle,
    session: &Session,
    start: i32,
    length: i32,
) -> Result<StringArray> {
    let mut handles = vec![0; length as usize];
    unsafe {
        raw::HAPI_GetParmStringValues(
            session.ptr(),
            node.0,
            1,
            handles.as_mut_ptr(),
            start,
            length,
        )
        .check_err(Some(session))?
    }
    crate::stringhandle::get_string_array(&handles, session)
}

pub fn get_parm_float_value(node: &HoudiniNode, name: &CStr, index: i32) -> Result<f32> {
    let mut value = uninit!();

    unsafe {
        raw::HAPI_GetParmFloatValue(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            index,
            value.as_mut_ptr(),
        )
        .check_err(Some(&node.session))?;
        Ok(value.assume_init())
    }
}

pub fn get_parm_int_value(node: &HoudiniNode, name: &CStr, index: i32) -> Result<i32> {
    let mut value = uninit!();

    unsafe {
        raw::HAPI_GetParmIntValue(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            index,
            value.as_mut_ptr(),
        )
        .check_err(Some(&node.session))?;
        Ok(value.assume_init())
    }
}

pub fn get_parm_string_value(node: &HoudiniNode, name: &CStr, index: i32) -> Result<String> {
    let mut handle = uninit!();
    let handle = unsafe {
        raw::HAPI_GetParmStringValue(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            index,
            1,
            handle.as_mut_ptr(),
        )
        .check_err(Some(&node.session))?;
        handle.assume_init()
    };
    node.session.get_string(handle)
}

pub fn get_parm_node_value(node: &HoudiniNode, name: &CStr) -> Result<Option<NodeHandle>> {
    unsafe {
        let mut id = uninit!();
        raw::HAPI_GetParmNodeValue(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            id.as_mut_ptr(),
        )
        .check_err(Some(&node.session))?;
        let id = id.assume_init();
        Ok(if id == -1 {
            None
        } else {
            Some(NodeHandle(id, ()))
        })
    }
}

pub fn set_parm_float_value(node: &HoudiniNode, name: &CStr, index: i32, value: f32) -> Result<()> {
    unsafe {
        raw::HAPI_SetParmFloatValue(
            node.session.ptr(),
            node.handle.0,
            name.as_ptr(),
            index,
            value,
        )
        .check_err(Some(&node.session))
    }
}

pub fn set_parm_float_values(
    node: NodeHandle,
    session: &Session,
    start: i32,
    length: i32,
    values: &[f32],
) -> Result<()> {
    if values.len() as i32 > length {
        log::warn!("Array length is greater than parm length: {:?}", values);
    }
    let length = values.len().min(length as usize);
    unsafe {
        raw::HAPI_SetParmFloatValues(session.ptr(), node.0, values.as_ptr(), start, length as i32)
            .check_err(Some(session))
    }
}

pub fn set_parm_int_values(
    node: NodeHandle,
    session: &Session,
    start: i32,
    length: i32,
    values: &[i32],
) -> Result<()> {
    unsafe {
        raw::HAPI_SetParmIntValues(session.ptr(), node.0, values.as_ptr(), start, length)
            .check_err(Some(session))
    }
}

pub fn set_parm_int_value(
    node: NodeHandle,
    session: &Session,
    name: &CStr,
    index: i32,
    value: i32,
) -> Result<()> {
    unsafe {
        raw::HAPI_SetParmIntValue(session.ptr(), node.0, name.as_ptr(), index, value)
            .check_err(Some(session))
    }
}

pub fn set_parm_string_value(
    node: NodeHandle,
    session: &Session,
    parm: &ParmHandle,
    index: i32,
    value: &CStr,
) -> Result<()> {
    unsafe {
        raw::HAPI_SetParmStringValue(session.ptr(), node.0, value.as_ptr(), parm.0, index)
            .check_err(Some(session))
    }
}

pub fn set_parm_string_values<T>(
    node: NodeHandle,
    session: &Session,
    parm: &ParmHandle,
    values: &[T],
) -> Result<()>
where
    T: AsRef<CStr>,
{
    for (i, v) in values.iter().enumerate() {
        set_parm_string_value(node, session, parm, i as i32, v.as_ref())?;
    }
    Ok(())
}

pub fn get_parm_choice_list(
    node: NodeHandle,
    session: &Session,
    index: i32,
    length: i32,
) -> Result<Vec<raw::HAPI_ParmChoiceInfo>> {
    unsafe {
        let mut structs = vec![raw::HAPI_ParmChoiceInfo_Create(); length as usize];
        raw::HAPI_GetParmChoiceLists(session.ptr(), node.0, structs.as_mut_ptr(), index, length)
            .check_err(Some(session))?;
        Ok(structs)
    }
}

pub fn get_parm_expression(
    node: NodeHandle,
    session: &Session,
    parm: &CStr,
    index: i32,
) -> Result<String> {
    let handle = unsafe {
        let mut handle = uninit!();
        raw::HAPI_GetParmExpression(
            session.ptr(),
            node.0,
            parm.as_ptr(),
            index,
            handle.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        handle.assume_init()
    };
    crate::stringhandle::get_string(handle, session)
}

pub fn parm_has_expression(
    node: NodeHandle,
    session: &Session,
    parm: &CStr,
    index: i32,
) -> Result<bool> {
    let ret = unsafe {
        let mut ret = uninit!();
        raw::HAPI_ParmHasExpression(
            session.ptr(),
            node.0,
            parm.as_ptr(),
            index,
            ret.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        ret.assume_init()
    };
    Ok(ret > 0)
}

pub fn set_parm_expression(
    node: NodeHandle,
    session: &Session,
    parm: ParmHandle,
    value: &CStr,
    index: i32,
) -> Result<()> {
    unsafe {
        raw::HAPI_SetParmExpression(session.ptr(), node.0, value.as_ptr(), parm.0, index)
            .check_err(Some(session))
    }
}

pub fn get_parm_info(
    node: NodeHandle,
    session: &Session,
    parm: ParmHandle,
) -> Result<raw::HAPI_ParmInfo> {
    unsafe {
        let mut info = uninit!();
        super::raw::HAPI_GetParmInfo(session.ptr(), node.0, parm.0, info.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(info.assume_init())
    }
}

pub fn get_parm_info_from_name(
    node: NodeHandle,
    session: &Session,
    name: &CStr,
) -> Result<raw::HAPI_ParmInfo> {
    unsafe {
        let mut info = uninit!();
        super::raw::HAPI_GetParmInfoFromName(
            session.ptr(),
            node.0,
            name.as_ptr(),
            info.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        Ok(info.assume_init())
    }
}

pub fn get_parm_id_from_name(name: &CStr, node: NodeHandle, session: &Session) -> Result<i32> {
    unsafe {
        let mut id = uninit!();
        crate::ffi::raw::HAPI_GetParmIdFromName(
            session.ptr(),
            node.0,
            name.as_ptr(),
            id.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        Ok(id.assume_init())
    }
}

pub fn get_node_info(node: NodeHandle, session: &Session) -> Result<raw::HAPI_NodeInfo> {
    unsafe {
        let mut info = uninit!();
        super::raw::HAPI_GetNodeInfo(session.ptr(), node.0, info.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(info.assume_init())
    }
}

pub fn is_node_valid(session: &Session, info: &raw::HAPI_NodeInfo) -> Result<bool> {
    unsafe {
        let mut answer = uninit!();
        raw::HAPI_IsNodeValid(
            session.ptr(),
            info.id,
            info.uniqueHoudiniNodeId,
            answer.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        Ok(answer.assume_init() == 1)
    }
}

pub fn delete_node(node: HoudiniNode) -> Result<()> {
    unsafe {
        raw::HAPI_DeleteNode(node.session.ptr(), node.handle.0).check_err(Some(&node.session))
    }
}

pub fn get_node_path(
    session: &Session,
    node: NodeHandle,
    relative_to: Option<NodeHandle>,
) -> Result<String> {
    unsafe {
        let mut sh = uninit!();
        raw::HAPI_GetNodePath(
            session.ptr(),
            node.0,
            relative_to.map(|n| n.0).unwrap_or(-1),
            sh.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        crate::stringhandle::get_string(sh.assume_init(), session)
    }
}

pub fn cook_node(node: &HoudiniNode, options: &CookOptions) -> Result<()> {
    unsafe {
        raw::HAPI_CookNode(node.session.ptr(), node.handle.0, options.ptr())
            .check_err(Some(&node.session))
    }
}

pub fn load_library_from_file(path: &CStr, session: &Session, _override: bool) -> Result<i32> {
    unsafe {
        let mut lib_id = uninit!();
        raw::HAPI_LoadAssetLibraryFromFile(
            session.ptr(),
            path.as_ptr(),
            _override as i8,
            lib_id.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        Ok(lib_id.assume_init())
    }
}

pub fn get_asset_info(node: &HoudiniNode) -> Result<raw::HAPI_AssetInfo> {
    unsafe {
        let mut info = uninit!();
        raw::HAPI_GetAssetInfo(node.session.ptr(), node.handle.0, info.as_mut_ptr())
            .check_err(Some(&node.session))?;
        Ok(info.assume_init())
    }
}

pub fn get_asset_count(library_id: i32, session: &Session) -> Result<i32> {
    unsafe {
        let mut num_assets = uninit!();
        raw::HAPI_GetAvailableAssetCount(session.ptr(), library_id, num_assets.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(num_assets.assume_init())
    }
}

pub fn get_asset_names(library_id: i32, num_assets: i32, session: &Session) -> Result<StringArray> {
    let handles = unsafe {
        let mut names = vec![0; num_assets as usize];
        raw::HAPI_GetAvailableAssets(session.ptr(), library_id, names.as_mut_ptr(), num_assets)
            .check_err(Some(session))?;
        names
    };
    crate::stringhandle::get_string_array(&handles, session)
}

#[derive(Default, Debug)]
pub struct ParmValueCount {
    pub parm_count: i32,
    pub int_count: i32,
    pub float_count: i32,
    pub string_count: i32,
    pub choice_count: i32,
}

pub fn get_asset_def_parm_count(
    library_id: i32,
    asset: &CStr,
    session: &Session,
) -> Result<ParmValueCount> {
    let mut parms = ParmValueCount::default();
    unsafe {
        raw::HAPI_GetAssetDefinitionParmCounts(
            session.ptr(),
            library_id,
            asset.as_ptr(),
            &mut parms.parm_count as *mut _,
            &mut parms.int_count as *mut _,
            &mut parms.float_count as *mut _,
            &mut parms.string_count as *mut _,
            &mut parms.choice_count as *mut _,
        )
        .check_err(Some(session))?;
    }
    Ok(parms)
}

pub fn get_asset_def_parm_info(
    library_id: i32,
    asset: &CStr,
    count: i32,
    session: &Session,
) -> Result<Vec<raw::HAPI_ParmInfo>> {
    unsafe {
        let mut parms = vec![raw::HAPI_ParmInfo_Create(); count as usize];
        raw::HAPI_GetAssetDefinitionParmInfos(
            session.ptr(),
            library_id,
            asset.as_ptr(),
            parms.as_mut_ptr(),
            0,
            count,
        )
        .check_err(Some(session))?;
        Ok(parms)
    }
}

#[allow(clippy::type_complexity)]
pub fn get_asset_def_parm_values(
    library_id: i32,
    asset: &CStr,
    session: &Session,
    count: &ParmValueCount,
) -> Result<(
    Vec<i32>,
    Vec<f32>,
    Vec<String>,
    Vec<raw::HAPI_ParmChoiceInfo>,
)> {
    let mut int_values = vec![0; count.int_count as usize];
    let mut float_values = vec![0.0; count.float_count as usize];
    let mut string_handles = vec![0; count.string_count as usize];
    let mut choice_values =
        vec![unsafe { raw::HAPI_ParmChoiceInfo_Create() }; count.choice_count as usize];
    unsafe {
        raw::HAPI_GetAssetDefinitionParmValues(
            session.ptr(),
            library_id,
            asset.as_ptr(),
            int_values.as_mut_ptr(),
            0,
            count.int_count,
            float_values.as_mut_ptr(),
            0,
            count.float_count,
            false as i8,
            string_handles.as_mut_ptr(),
            0,
            count.string_count,
            choice_values.as_mut_ptr(),
            0,
            count.choice_count,
        )
        .check_err(Some(session))?;
    }

    let string_array = crate::stringhandle::get_string_array(&string_handles, session)?;
    let string_values = string_array.into_iter().collect();
    Ok((int_values, float_values, string_values, choice_values))
}

pub fn get_string_batch_size(handles: &[i32], session: &Session) -> Result<i32> {
    unsafe {
        let mut length = uninit!();
        raw::HAPI_GetStringBatchSize(
            session.ptr(),
            handles.as_ptr(),
            handles.len() as i32,
            length.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        Ok(length.assume_init())
    }
}

/// Note: contiguous array of null-terminated strings
pub fn get_string_batch(length: i32, session: &Session) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; length as usize];
    unsafe {
        raw::HAPI_GetStringBatch(session.ptr(), buffer.as_mut_ptr() as *mut _, length as i32)
            .check_err(Some(session))?;
    }
    buffer.truncate(length as usize);
    Ok(buffer)
}

pub fn get_string_buff_len(session: &Session, handle: i32) -> Result<i32> {
    unsafe {
        let mut length = uninit!();
        raw::HAPI_GetStringBufLength(session.ptr(), handle, length.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(length.assume_init())
    }
}

pub fn get_string(session: &Session, handle: i32, length: i32) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; length as usize];
    unsafe {
        raw::HAPI_GetString(session.ptr(), handle, buffer.as_mut_ptr() as *mut _, length)
            .check_err(None)?;
        buffer.truncate(length as usize - 1);
    }
    Ok(buffer)
}

pub fn get_status_string(
    session: &Session,
    status: raw::StatusType,
    verbosity: raw::StatusVerbosity,
) -> Result<String> {
    let mut length = uninit!();
    let _lock = session.handle.1.lock();
    unsafe {
        raw::HAPI_GetStatusStringBufLength(session.ptr(), status, verbosity, length.as_mut_ptr())
            .error_message("GetStatusStringBufLength failed")?;
        let length = length.assume_init();
        let mut buf = vec![0u8; length as usize];
        if length > 0 {
            raw::HAPI_GetStatusString(session.ptr(), status, buf.as_mut_ptr() as *mut i8, length)
                .error_message("GetStatusString failed")?;
            buf.truncate(length as usize - 1);
            Ok(String::from_utf8_unchecked(buf))
        } else {
            Ok(String::new())
        }
    }
}

pub fn create_inprocess_session() -> Result<raw::HAPI_Session> {
    let mut ses = uninit!();
    unsafe {
        match raw::HAPI_CreateInProcessSession(ses.as_mut_ptr()) {
            err @ raw::HapiResult::Failure => Err(HapiError::new(
                Kind::Hapi(err),
                None,
                Some(get_connection_error(true)?.into()),
            )),
            _ => Ok(ses.assume_init()),
        }
    }
}

pub fn set_server_env_str(session: &Session, key: &CStr, value: &CStr) -> Result<()> {
    unsafe {
        raw::HAPI_SetServerEnvString(session.ptr(), key.as_ptr(), value.as_ptr())
            .check_err(Some(session))
    }
}

pub fn set_server_env_int(session: &Session, key: &CStr, value: i32) -> Result<()> {
    unsafe {
        raw::HAPI_SetServerEnvInt(session.ptr(), key.as_ptr(), value).check_err(Some(session))
    }
}

pub fn get_server_env_var_count(session: &Session) -> Result<i32> {
    unsafe {
        let mut val = uninit!();
        raw::HAPI_GetServerEnvVarCount(session.ptr(), val.as_mut_ptr()).check_err(Some(session))?;
        Ok(val.assume_init())
    }
}

pub fn get_server_env_var_list(session: &Session, count: i32) -> Result<Vec<i32>> {
    unsafe {
        let mut handles = vec![0; count as usize];
        raw::HAPI_GetServerEnvVarList(session.ptr(), handles.as_mut_ptr(), 0, count)
            .check_err(Some(session))?;
        Ok(handles)
    }
}

pub fn get_server_env_str(session: &Session, key: &CStr) -> Result<i32> {
    unsafe {
        let mut val = uninit!();
        raw::HAPI_GetServerEnvString(session.ptr(), key.as_ptr(), val.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(val.assume_init())
    }
}

pub fn get_server_env_int(session: &Session, key: &CStr) -> Result<i32> {
    unsafe {
        let mut val = uninit!();
        raw::HAPI_GetServerEnvInt(session.ptr(), key.as_ptr(), val.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(val.assume_init())
    }
}

pub fn start_thrift_pipe_server(
    file: &CStr,
    options: &raw::HAPI_ThriftServerOptions,
) -> Result<u32> {
    let mut pid = uninit!();
    unsafe {
        raw::HAPI_StartThriftNamedPipeServer(options as *const _, file.as_ptr(), pid.as_mut_ptr())
            .error_message("Could not start thrift server")?;
        Ok(pid.assume_init())
    }
}

pub fn start_thrift_socket_server(
    port: i32,
    options: &raw::HAPI_ThriftServerOptions,
) -> Result<u32> {
    let mut pid = uninit!();
    unsafe {
        raw::HAPI_StartThriftSocketServer(options as *const _, port, pid.as_mut_ptr())
            .error_message("Could not start thrift server")?;
        Ok(pid.assume_init())
    }
}

pub fn new_thrift_piped_session(path: &CStr) -> Result<raw::HAPI_Session> {
    let mut handle = uninit!();
    let session = unsafe {
        raw::HAPI_CreateThriftNamedPipeSession(handle.as_mut_ptr(), path.as_ptr())
            .error_message("Could not start piped session")?;
        handle.assume_init()
    };
    Ok(session)
}

pub fn new_thrift_socket_session(port: i32, host: &CStr) -> Result<raw::HAPI_Session> {
    let mut handle = uninit!();
    let session = unsafe {
        raw::HAPI_CreateThriftSocketSession(handle.as_mut_ptr(), host.as_ptr(), port)
            .error_message("Could not start socket session")?;
        handle.assume_init()
    };
    Ok(session)
}

pub fn initialize_session(session: &Session, options: &SessionOptions) -> Result<()> {
    unsafe {
        let res = raw::HAPI_Initialize(
            session.ptr(),
            options.cook_opt.ptr(),
            options.threaded as i8,
            -1,
            options
                .env_files
                .as_ref()
                .map(|p| p.as_ptr())
                .unwrap_or(null()),
            options
                .otl_path
                .as_ref()
                .map(|p| p.as_ptr())
                .unwrap_or(null()),
            options
                .dso_path
                .as_ref()
                .map(|p| p.as_ptr())
                .unwrap_or(null()),
            options
                .img_dso_path
                .as_ref()
                .map(|p| p.as_ptr())
                .unwrap_or(null()),
            options
                .aud_dso_path
                .as_ref()
                .map(|p| p.as_ptr())
                .unwrap_or(null()),
        );
        match is_session_valid(session) {
            true => res.check_err(Some(session)),
            false => res.error_message("Could not initialize session"),
        }
    }
}

pub fn cleanup_session(session: &Session) -> Result<()> {
    unsafe { raw::HAPI_Cleanup(session.ptr()).check_err(Some(session)) }
}

pub fn close_session(session: &Session) -> Result<()> {
    unsafe { raw::HAPI_CloseSession(session.ptr()).check_err(Some(session)) }
}

pub fn is_session_initialized(session: &Session) -> bool {
    unsafe {
        match raw::HAPI_IsInitialized(session.ptr()) {
            raw::HapiResult::Success => true,
            raw::HapiResult::NotInitialized => false,
            e => panic!("HAPI_IsInitialized error: {:?}", e),
        }
    }
}

pub fn save_hip(session: &Session, name: &CStr, lock_nodes: bool) -> Result<()> {
    unsafe {
        raw::HAPI_SaveHIPFile(session.ptr(), name.as_ptr(), lock_nodes as i8)
            .check_err(Some(session))
    }
}

pub fn load_hip(session: &Session, name: &CStr, cook: bool) -> Result<()> {
    unsafe {
        raw::HAPI_LoadHIPFile(session.ptr(), name.as_ptr(), cook as i8).check_err(Some(session))
    }
}

pub fn merge_hip(session: &Session, name: &CStr, cook: bool) -> Result<i32> {
    unsafe {
        let mut id = uninit!();
        raw::HAPI_MergeHIPFile(session.ptr(), name.as_ptr(), cook as i8, id.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(id.assume_init())
    }
}

pub fn interrupt(session: &Session) -> Result<()> {
    unsafe { raw::HAPI_Interrupt(session.ptr()).check_err(Some(session)) }
}

pub fn get_status(session: &Session, flag: raw::StatusType) -> Result<raw::State> {
    let status = unsafe {
        let mut status = uninit!();
        raw::HAPI_GetStatus(session.ptr(), flag, status.as_mut_ptr()).check_err(Some(session))?;
        status.assume_init()
    };
    Ok(raw::State::from(status))
}

pub fn is_session_valid(session: &Session) -> bool {
    unsafe {
        matches!(
            raw::HAPI_IsSessionValid(session.ptr()),
            raw::HapiResult::Success
        )
    }
}

pub fn get_cooking_total_count(session: &Session) -> Result<i32> {
    unsafe {
        let mut count = uninit!();
        raw::HAPI_GetCookingTotalCount(session.ptr(), count.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(count.assume_init())
    }
}

pub fn get_cooking_current_count(session: &Session) -> Result<i32> {
    unsafe {
        let mut count = uninit!();
        raw::HAPI_GetCookingCurrentCount(session.ptr(), count.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(count.assume_init())
    }
}

pub fn get_connection_error(clear: bool) -> Result<String> {
    unsafe {
        let mut length = uninit!();
        raw::HAPI_GetConnectionErrorLength(length.as_mut_ptr())
            .error_message("HAPI_GetConnectionErrorLength failed")?;
        let length = length.assume_init();
        if length > 0 {
            let mut buf = vec![0u8; length as usize];
            raw::HAPI_GetConnectionError(buf.as_mut_ptr() as *mut _, length, clear as i8)
                .error_message("HAPI_GetConnectionError failed")?;
            Ok(String::from_utf8_unchecked(buf))
        } else {
            Ok(String::new())
        }
    }
}

pub fn get_total_cook_count(
    node: &HoudiniNode,
    node_types: raw::NodeType,
    node_flags: raw::NodeFlags,
    recursive: bool,
) -> Result<i32> {
    let mut count = uninit!();
    unsafe {
        raw::HAPI_GetTotalCookCount(
            node.session.ptr(),
            node.handle.0,
            node_types.0,
            node_flags.0,
            recursive as i8,
            count.as_mut_ptr(),
        )
        .check_err(Some(&node.session))?;
        Ok(count.assume_init())
    }
}

pub fn create_node(
    name: &CStr,
    label: Option<&CStr>,
    session: &Session,
    parent: Option<NodeHandle>,
    cook: bool,
) -> Result<raw::HAPI_NodeId> {
    unsafe {
        let mut id = uninit!();
        raw::HAPI_CreateNode(
            session.ptr(),
            parent.map_or(-1, |h| h.0),
            name.as_ptr(),
            label.map_or(null(), |v| v.as_ptr()),
            cook as i8,
            id.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        Ok(id.assume_init())
    }
}

pub fn create_input_node(session: &Session, name: &CStr) -> Result<raw::HAPI_NodeId> {
    let mut id = uninit!();
    unsafe {
        raw::HAPI_CreateInputNode(session.ptr(), id.as_mut_ptr(), name.as_ptr())
            .check_err(Some(session))?;
        Ok(id.assume_init())
    }
}

pub fn get_manager_node(session: &Session, node_type: raw::NodeType) -> Result<raw::HAPI_NodeId> {
    unsafe {
        let mut id = uninit!();
        raw::HAPI_GetManagerNodeId(session.ptr(), node_type, id.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(id.assume_init())
    }
}

pub fn get_compose_child_node_list(
    node: &HoudiniNode,
    types: raw::NodeType,
    flags: raw::NodeFlags,
    recursive: bool,
) -> Result<Vec<i32>> {
    unsafe {
        let mut count = uninit!();
        raw::HAPI_ComposeChildNodeList(
            node.session.ptr(),
            node.handle.0,
            types.0,
            flags.0,
            recursive as i8,
            count.as_mut_ptr(),
        )
        .check_err(Some(&node.session))?;

        let count = count.assume_init();
        let mut obj_infos = vec![0i32; count as usize];
        raw::HAPI_GetComposedChildNodeList(
            node.session.ptr(),
            node.handle.0,
            obj_infos.as_mut_ptr(),
            count,
        )
        .check_err(Some(&node.session))?;
        Ok(obj_infos)
    }
}

pub fn get_composed_object_list(
    session: &Session,
    parent: NodeHandle,
) -> Result<Vec<raw::HAPI_ObjectInfo>> {
    unsafe {
        let mut count = uninit!();
        raw::HAPI_ComposeObjectList(session.ptr(), parent.0, null(), count.as_mut_ptr())
            .check_err(Some(session))?;
        let count = count.assume_init();
        let mut obj_infos = vec![raw::HAPI_ObjectInfo_Create(); count as usize];
        raw::HAPI_GetComposedObjectList(session.ptr(), parent.0, obj_infos.as_mut_ptr(), 0, count)
            .check_err(Some(session))?;
        Ok(obj_infos)
    }
}

pub fn get_parameters(node: &HoudiniNode) -> Result<Vec<raw::HAPI_ParmInfo>> {
    unsafe {
        let mut parms = vec![raw::HAPI_ParmInfo_Create(); node.info.parm_count() as usize];
        raw::HAPI_GetParameters(
            node.session.ptr(),
            node.handle.0,
            parms.as_mut_ptr(),
            0,
            node.info.parm_count(),
        )
        .check_err(Some(&node.session))?;
        Ok(parms)
    }
}

pub fn connect_node_input(
    session: &Session,
    node_id: NodeHandle,
    input_index: i32,
    node_id_to_connect: NodeHandle,
    output_index: i32,
) -> Result<()> {
    unsafe {
        raw::HAPI_ConnectNodeInput(
            session.ptr(),
            node_id.0,
            input_index,
            node_id_to_connect.0,
            output_index,
        )
        .check_err(Some(session))
    }
}

pub fn query_node_input(node: &HoudiniNode, idx: i32) -> Result<i32> {
    let mut inp_idx = uninit!();
    unsafe {
        raw::HAPI_QueryNodeInput(node.session.ptr(), node.handle.0, idx, inp_idx.as_mut_ptr())
            .check_err(Some(&node.session))?;
        Ok(inp_idx.assume_init())
    }
}

pub fn check_for_specific_errors(
    node: &HoudiniNode,
    error_bits: raw::ErrorCode,
) -> Result<raw::ErrorCode> {
    unsafe {
        let mut code = uninit!();
        raw::HAPI_CheckForSpecificErrors(
            node.session.ptr(),
            node.handle.0,
            error_bits.0 as i32,
            code.as_mut_ptr(),
        )
        .check_err(Some(&node.session))?;
        Ok(raw::ErrorCode(code.assume_init()))
    }
}

pub unsafe fn get_composed_cook_result(
    node: &HoudiniNode,
    verbosity: raw::StatusVerbosity,
) -> Result<String> {
    let mut len = uninit!();
    raw::HAPI_ComposeNodeCookResult(
        node.session.ptr(),
        node.handle.0,
        verbosity,
        len.as_mut_ptr(),
    )
    .check_err(Some(&node.session))?;
    let len = len.assume_init();
    let mut buf = vec![0u8; len as usize];
    raw::HAPI_GetComposedNodeCookResult(node.session.ptr(), buf.as_mut_ptr() as *mut i8, len)
        .check_err(Some(&node.session))?;
    buf.truncate(len as usize - 1);
    Ok(String::from_utf8_unchecked(buf))
}

pub fn get_time(session: &Session) -> Result<f32> {
    unsafe {
        let mut time = uninit!();
        raw::HAPI_GetTime(session.ptr(), time.as_mut_ptr()).check_err(Some(session))?;
        Ok(time.assume_init())
    }
}

pub fn set_time(session: &Session, time: f32) -> Result<()> {
    unsafe { raw::HAPI_SetTime(session.ptr(), time).check_err(Some(session)) }
}

pub fn set_timeline_options(session: &Session, options: &raw::HAPI_TimelineOptions) -> Result<()> {
    unsafe {
        raw::HAPI_SetTimelineOptions(session.ptr(), options as *const _).check_err(Some(session))
    }
}

pub fn get_timeline_options(session: &Session) -> Result<raw::HAPI_TimelineOptions> {
    unsafe {
        let mut opt = uninit!();
        raw::HAPI_GetTimelineOptions(session.ptr(), opt.as_mut_ptr()).check_err(Some(session))?;
        Ok(opt.assume_init())
    }
}

pub fn set_use_houdini_time(session: &Session, do_use: bool) -> Result<()> {
    unsafe { raw::HAPI_SetUseHoudiniTime(session.ptr(), do_use as i8).check_err(Some(session)) }
}

pub fn reset_simulation(node: &HoudiniNode) -> Result<()> {
    unsafe {
        raw::HAPI_ResetSimulation(node.session.ptr(), node.handle.0).check_err(Some(&node.session))
    }
}

pub fn get_hipfile_node_count(session: &Session, hip_file_id: i32) -> Result<u32> {
    unsafe {
        let mut count = uninit!();
        raw::HAPI_GetHIPFileNodeCount(session.ptr(), hip_file_id, count.as_mut_ptr());
        Ok(count.assume_init() as u32)
    }
}

pub fn get_geo_display_info(node: &HoudiniNode) -> Result<raw::HAPI_GeoInfo> {
    unsafe {
        let mut info = uninit!();
        raw::HAPI_GetDisplayGeoInfo(node.session.ptr(), node.handle.0, info.as_mut_ptr())
            .check_err(Some(&node.session))?;
        Ok(info.assume_init())
    }
}

pub fn get_geo_info(session: &Session, node: NodeHandle) -> Result<raw::HAPI_GeoInfo> {
    unsafe {
        let mut info = uninit!();
        raw::HAPI_GetGeoInfo(session.ptr(), node.0, info.as_mut_ptr()).check_err(Some(session))?;
        Ok(info.assume_init())
    }
}

pub fn get_output_geo_count(node: &HoudiniNode) -> Result<i32> {
    let mut count = uninit!();
    unsafe {
        raw::HAPI_GetOutputGeoCount(node.session.ptr(), node.handle.0, count.as_mut_ptr())
            .check_err(Some(&node.session))?;
        Ok(count.assume_init())
    }
}

pub fn get_output_geos(node: &HoudiniNode) -> Result<Vec<raw::HAPI_GeoInfo>> {
    let count = get_output_geo_count(node)?;
    unsafe {
        let mut obj_infos = vec![raw::HAPI_GeoInfo_Create(); count as usize];
        raw::HAPI_GetOutputGeoInfos(
            node.session.ptr(),
            node.handle.0,
            obj_infos.as_mut_ptr(),
            count,
        )
        .check_err(Some(&node.session))?;
        Ok(obj_infos)
    }
}

pub fn get_group_count_by_type(geo_info: &GeoInfo, group_type: raw::GroupType) -> i32 {
    // SAFETY: Not sure why but many HAPI functions take a mutable pointer where they
    // actually shouldn't?
    let ptr = (&geo_info.inner as *const _) as *mut raw::HAPI_GeoInfo;
    unsafe { raw::HAPI_GeoInfo_GetGroupCountByType(ptr, group_type) }
}

pub fn get_edge_count_of_edge_group(
    session: &Session,
    node: NodeHandle,
    group_name: &CStr,
    part_id: i32,
) -> Result<i32> {
    let mut count = uninit!();
    unsafe {
        raw::HAPI_GetEdgeCountOfEdgeGroup(
            session.ptr(),
            node.0,
            part_id,
            group_name.as_ptr(),
            count.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        Ok(count.assume_init())
    }
}

pub fn get_part_info(node: &HoudiniNode, id: i32) -> Result<raw::HAPI_PartInfo> {
    unsafe {
        let mut info = uninit!();
        super::raw::HAPI_GetPartInfo(node.session.ptr(), node.handle.0, id, info.as_mut_ptr())
            .check_err(Some(&node.session))?;
        Ok(info.assume_init())
    }
}

pub fn get_volume_info(node: &HoudiniNode, id: i32) -> Result<raw::HAPI_VolumeInfo> {
    unsafe {
        let mut info = uninit!();
        super::raw::HAPI_GetVolumeInfo(node.session.ptr(), node.handle.0, id, info.as_mut_ptr())
            .check_err(Some(&node.session))?;
        Ok(info.assume_init())
    }
}

pub fn set_volume_info(node: &HoudiniNode, part: i32, info: &raw::HAPI_VolumeInfo) -> Result<()> {
    unsafe {
        super::raw::HAPI_SetVolumeInfo(node.session.ptr(), node.handle.0, part, info)
            .check_err(Some(&node.session))
    }
}

pub fn get_volume_first_tile_info(node: &HoudiniNode, id: i32) -> Result<raw::HAPI_VolumeTileInfo> {
    unsafe {
        let mut info = uninit!();
        super::raw::HAPI_GetFirstVolumeTile(
            node.session.ptr(),
            node.handle.0,
            id,
            info.as_mut_ptr(),
        )
        .check_err(Some(&node.session))?;
        Ok(info.assume_init())
    }
}

pub fn get_volume_next_tile_info(
    node: &HoudiniNode,
    id: i32,
    tile: &mut raw::HAPI_VolumeTileInfo,
) -> Result<()> {
    unsafe {
        super::raw::HAPI_GetNextVolumeTile(node.session.ptr(), node.handle.0, id, tile)
            .check_err(Some(&node.session))?;
        Ok(())
    }
}

pub fn get_volume_tile_float_data(
    node: &HoudiniNode,
    part: i32,
    fill_value: f32,
    values: &mut [f32],
    tile: &raw::HAPI_VolumeTileInfo,
) -> Result<()> {
    unsafe {
        raw::HAPI_GetVolumeTileFloatData(
            node.session.ptr(),
            node.handle.0,
            part,
            fill_value,
            tile,
            values.as_mut_ptr(),
            values.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn set_volume_tile_float_data(
    node: &HoudiniNode,
    part: i32,
    values: &[f32],
    tile: &raw::HAPI_VolumeTileInfo,
) -> Result<()> {
    unsafe {
        raw::HAPI_SetVolumeTileFloatData(
            node.session.ptr(),
            node.handle.0,
            part,
            tile,
            values.as_ptr(),
            values.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn set_volume_tile_int_data(
    node: &HoudiniNode,
    part: i32,
    values: &[i32],
    tile: &raw::HAPI_VolumeTileInfo,
) -> Result<()> {
    unsafe {
        raw::HAPI_SetVolumeTileIntData(
            node.session.ptr(),
            node.handle.0,
            part,
            tile,
            values.as_ptr(),
            values.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn get_volume_tile_int_data(
    node: &HoudiniNode,
    part: i32,
    fill_value: i32,
    values: &mut [i32],
    tile: &raw::HAPI_VolumeTileInfo,
) -> Result<()> {
    unsafe {
        raw::HAPI_GetVolumeTileIntData(
            node.session.ptr(),
            node.handle.0,
            part,
            fill_value,
            tile,
            values.as_mut_ptr(),
            values.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn get_volume_voxel_int(
    node: &HoudiniNode,
    part: i32,
    x: i32,
    y: i32,
    z: i32,
    values: &mut [i32],
) -> Result<()> {
    unsafe {
        raw::HAPI_GetVolumeVoxelIntData(
            node.session.ptr(),
            node.handle.0,
            part,
            x,
            y,
            z,
            values.as_mut_ptr(),
            values.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn set_volume_voxel_int(
    node: &HoudiniNode,
    part: i32,
    x: i32,
    y: i32,
    z: i32,
    values: &[i32],
) -> Result<()> {
    unsafe {
        raw::HAPI_SetVolumeVoxelIntData(
            node.session.ptr(),
            node.handle.0,
            part,
            x,
            y,
            z,
            values.as_ptr(),
            values.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn get_volume_voxel_float(
    node: &HoudiniNode,
    part: i32,
    x: i32,
    y: i32,
    z: i32,
    values: &mut [f32],
) -> Result<()> {
    unsafe {
        raw::HAPI_GetVolumeVoxelFloatData(
            node.session.ptr(),
            node.handle.0,
            part,
            x,
            y,
            z,
            values.as_mut_ptr(),
            values.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn set_volume_voxel_float(
    node: &HoudiniNode,
    part: i32,
    x: i32,
    y: i32,
    z: i32,
    values: &[f32],
) -> Result<()> {
    unsafe {
        raw::HAPI_SetVolumeVoxelFloatData(
            node.session.ptr(),
            node.handle.0,
            part,
            x,
            y,
            z,
            values.as_ptr(),
            values.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn get_volume_bounds(node: &HoudiniNode, id: i32) -> Result<crate::volume::VolumeBounds> {
    unsafe {
        let mut b = crate::volume::VolumeBounds::default();
        super::raw::HAPI_GetVolumeBounds(
            node.session.ptr(),
            node.handle.0,
            id,
            &mut b.x_min as *mut _,
            &mut b.y_min as *mut _,
            &mut b.z_min as *mut _,
            &mut b.x_max as *mut _,
            &mut b.y_max as *mut _,
            &mut b.z_max as *mut _,
            &mut b.x_center as *mut _,
            &mut b.y_center as *mut _,
            &mut b.z_center as *mut _,
        )
        .check_err(Some(&node.session))?;
        Ok(b)
    }
}

pub fn create_heightfield_input(
    node: &HoudiniNode,
    parent: Option<i32>,
    name: &CStr,
    x_size: i32,
    y_size: i32,
    voxel_size: f32,
    sampling: raw::HeightFieldSampling,
) -> Result<(i32, i32, i32, i32)> {
    let heightfield_node = -1;
    let height_node = -1;
    let mask_node = -1;
    let merge_node = -1;
    unsafe {
        raw::HAPI_CreateHeightFieldInput(
            node.session.ptr(),
            parent.unwrap_or(-1),
            name.as_ptr(),
            x_size,
            y_size,
            voxel_size,
            sampling,
            heightfield_node as *mut _,
            height_node as *mut _,
            mask_node as *mut _,
            merge_node as *mut _,
        )
        .check_err(Some(&node.session))?;
    }
    Ok((heightfield_node, height_node, mask_node, merge_node))
}

pub fn create_heightfield_input_volume(
    node: &HoudiniNode,
    parent: Option<i32>,
    name: &CStr,
    xsize: i32,
    ysize: i32,
    size: f32,
) -> Result<NodeHandle> {
    let volume_node = -1;
    unsafe {
        raw::HAPI_CreateHeightfieldInputVolumeNode(
            node.session.ptr(),
            parent.unwrap_or(-1),
            volume_node as *mut _,
            name.as_ptr(),
            xsize,
            ysize,
            size,
        )
        .check_err(Some(&node.session))?;
    }

    Ok(NodeHandle(volume_node, ()))
}

pub fn set_part_info(node: &HoudiniNode, info: &PartInfo) -> Result<()> {
    unsafe {
        super::raw::HAPI_SetPartInfo(
            node.session.ptr(),
            node.handle.0,
            info.part_id(),
            &info.inner,
        )
        .check_err(Some(&node.session))
    }
}

pub fn set_curve_info(node: &HoudiniNode, info: &CurveInfo, part_id: i32) -> Result<()> {
    unsafe {
        super::raw::HAPI_SetCurveInfo(node.session.ptr(), node.handle.0, part_id, &info.inner)
            .check_err(Some(&node.session))
    }
}

pub fn get_curve_info(node: &HoudiniNode, part_id: i32) -> Result<raw::HAPI_CurveInfo> {
    unsafe {
        let mut info = uninit!();
        super::raw::HAPI_GetCurveInfo(
            node.session.ptr(),
            node.handle.0,
            part_id,
            info.as_mut_ptr(),
        )
        .check_err(Some(&node.session))?;
        Ok(info.assume_init())
    }
}

pub fn get_curve_counts(
    node: &HoudiniNode,
    part_id: i32,
    start: i32,
    length: i32,
) -> Result<Vec<i32>> {
    unsafe {
        let mut array = vec![0; length as usize];
        raw::HAPI_GetCurveCounts(
            node.session.ptr(),
            node.handle.0,
            part_id,
            array.as_mut_ptr(),
            start,
            length,
        )
        .check_err(Some(&node.session))?;
        Ok(array)
    }
}

pub fn get_curve_orders(
    node: &HoudiniNode,
    part_id: i32,
    start: i32,
    length: i32,
) -> Result<Vec<i32>> {
    unsafe {
        let mut array = vec![0; length as usize];
        raw::HAPI_GetCurveOrders(
            node.session.ptr(),
            node.handle.0,
            part_id,
            array.as_mut_ptr(),
            start,
            length,
        )
        .check_err(Some(&node.session))?;
        Ok(array)
    }
}

pub fn get_curve_knots(
    node: &HoudiniNode,
    part_id: i32,
    start: i32,
    length: i32,
) -> Result<Vec<f32>> {
    unsafe {
        let mut array = vec![0.0; length as usize];
        raw::HAPI_GetCurveKnots(
            node.session.ptr(),
            node.handle.0,
            part_id,
            array.as_mut_ptr(),
            start,
            length,
        )
        .check_err(Some(&node.session))?;
        Ok(array)
    }
}

pub fn set_curve_counts(node: &HoudiniNode, part_id: i32, count: &[i32]) -> Result<()> {
    unsafe {
        super::raw::HAPI_SetCurveCounts(
            node.session.ptr(),
            node.handle.0,
            part_id,
            count.as_ptr(),
            0,
            count.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn set_curve_knots(node: &HoudiniNode, part_id: i32, knots: &[f32]) -> Result<()> {
    unsafe {
        super::raw::HAPI_SetCurveKnots(
            node.session.ptr(),
            node.handle.0,
            part_id,
            knots.as_ptr(),
            0,
            knots.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn set_curve_orders(node: &HoudiniNode, part_id: i32, knots: &[i32]) -> Result<()> {
    unsafe {
        super::raw::HAPI_SetCurveOrders(
            node.session.ptr(),
            node.handle.0,
            part_id,
            knots.as_ptr(),
            0,
            knots.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn get_box_info(
    node: NodeHandle,
    session: &Session,
    part_id: i32,
) -> Result<raw::HAPI_BoxInfo> {
    let mut info = raw::HAPI_BoxInfo {
        center: Default::default(),
        size: Default::default(),
        rotation: Default::default(),
    };
    unsafe {
        let box_info = &mut info as *mut _;
        raw::HAPI_GetBoxInfo(session.ptr(), node.0, part_id, box_info).check_err(Some(session))?;
    }
    Ok(info)
}

pub fn get_sphere_info(
    node: NodeHandle,
    session: &Session,
    part_id: i32,
) -> Result<raw::HAPI_SphereInfo> {
    let mut info = raw::HAPI_SphereInfo {
        center: Default::default(),
        radius: 0.0,
    };
    unsafe {
        let sphere_info = &mut info as *mut _;
        raw::HAPI_GetSphereInfo(session.ptr(), node.0, part_id, sphere_info)
            .check_err(Some(session))?;
    }
    Ok(info)
}

pub fn get_attribute_names(
    node: &HoudiniNode,
    part_id: i32,
    count: i32,
    owner: raw::AttributeOwner,
) -> Result<StringArray> {
    let mut handles = vec![0; count as usize];
    unsafe {
        raw::HAPI_GetAttributeNames(
            node.session.ptr(),
            node.handle.0,
            part_id,
            owner,
            handles.as_mut_ptr(),
            count,
        )
        .check_err(Some(&node.session))?;
    }
    crate::stringhandle::get_string_array(&handles, &node.session)
}

pub fn get_attribute_info(
    node: &HoudiniNode,
    part_id: i32,
    owner: raw::AttributeOwner,
    name: &CStr,
) -> Result<raw::HAPI_AttributeInfo> {
    let mut info = uninit!();
    unsafe {
        raw::HAPI_GetAttributeInfo(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            owner,
            info.as_mut_ptr(),
        )
        .check_err(Some(&node.session))?;

        Ok(info.assume_init())
    }
}

pub fn add_attribute(
    node: &HoudiniNode,
    part_id: i32,
    name: &CStr,
    attr_info: &raw::HAPI_AttributeInfo,
) -> Result<()> {
    unsafe {
        raw::HAPI_AddAttribute(
            node.session.ptr(),
            node.handle.0,
            part_id,
            name.as_ptr(),
            attr_info,
        )
        .check_err(Some(&node.session))
    }
}

pub fn get_face_counts(
    session: &Session,
    node: NodeHandle,
    part_id: i32,
    start: i32,
    length: i32,
) -> Result<Vec<i32>> {
    let mut array = vec![0; length as usize];
    unsafe {
        raw::HAPI_GetFaceCounts(
            session.ptr(),
            node.0,
            part_id,
            array.as_mut_ptr(),
            start,
            length,
        )
        .check_err(Some(session))?;
    }
    Ok(array)
}

pub fn get_element_count_by_group(part_info: &PartInfo, group_type: raw::GroupType) -> i32 {
    // SAFETY: Not sure why but many HAPI functions take a mutable pointer where they
    // actually shouldn't?
    let ptr = (&part_info.inner as *const raw::HAPI_PartInfo) as *mut raw::HAPI_PartInfo;
    unsafe { raw::HAPI_PartInfo_GetElementCountByGroupType(ptr, group_type) }
}

pub fn add_group(
    session: &Session,
    node: NodeHandle,
    part_id: i32,
    group_type: raw::GroupType,
    group_name: &CStr,
) -> Result<()> {
    unsafe {
        raw::HAPI_AddGroup(
            session.ptr(),
            node.0,
            part_id,
            group_type,
            group_name.as_ptr(),
        )
        .check_err(Some(session))
    }
}

pub fn delete_group(
    session: &Session,
    node: NodeHandle,
    part_id: i32,
    group_type: raw::GroupType,
    group_name: &CStr,
) -> Result<()> {
    unsafe {
        raw::HAPI_DeleteGroup(
            session.ptr(),
            node.0,
            part_id,
            group_type,
            group_name.as_ptr(),
        )
        .check_err(Some(session))
    }
}

pub fn set_group_membership(
    session: &Session,
    node: NodeHandle,
    part_id: i32,
    group_type: raw::GroupType,
    group_name: &CStr,
    array: &[i32],
) -> Result<()> {
    unsafe {
        raw::HAPI_SetGroupMembership(
            session.ptr(),
            node.0,
            part_id,
            group_type,
            group_name.as_ptr(),
            array.as_ptr(),
            0,
            array.len() as i32,
        )
        .check_err(Some(session))
    }
}

pub fn get_group_membership(
    session: &Session,
    node: NodeHandle,
    part_id: i32,
    group_type: raw::GroupType,
    group_name: &CStr,
    length: i32,
) -> Result<Vec<i32>> {
    unsafe {
        let mut length = length;
        if matches!(group_type, raw::GroupType::Edge) {
            length *= 2;
        }
        let mut array = vec![0; length as usize];
        raw::HAPI_GetGroupMembership(
            session.ptr(),
            node.0,
            part_id,
            group_type,
            group_name.as_ptr(),
            null_mut::<i8>(),
            array.as_mut_ptr(),
            0,
            length,
        )
        .check_err(Some(session))?;
        Ok(array)
    }
}

pub fn get_group_names(
    node: &HoudiniNode,
    group_type: raw::GroupType,
    count: i32,
) -> Result<StringArray> {
    let mut handles = vec![0; count as usize];
    unsafe {
        raw::HAPI_GetGroupNames(
            node.session.ptr(),
            node.handle.0,
            group_type,
            handles.as_mut_ptr(),
            count,
        )
        .check_err(Some(&node.session))?;
    }
    crate::stringhandle::get_string_array(&handles, &node.session)
}

pub fn save_geo_to_file(node: &HoudiniNode, filename: &CStr) -> Result<()> {
    unsafe {
        raw::HAPI_SaveGeoToFile(node.session.ptr(), node.handle.0, filename.as_ptr())
            .check_err(Some(&node.session))
    }
}

pub fn load_geo_from_file(node: &HoudiniNode, filename: &CStr) -> Result<()> {
    unsafe {
        raw::HAPI_LoadGeoFromFile(node.session.ptr(), node.handle.0, filename.as_ptr())
            .check_err(Some(&node.session))
    }
}

pub fn save_node_to_file(node: NodeHandle, session: &Session, filename: &CStr) -> Result<()> {
    unsafe {
        raw::HAPI_SaveNodeToFile(session.ptr(), node.0, filename.as_ptr()).check_err(Some(session))
    }
}

pub fn load_node_from_file(
    parent_node: Option<NodeHandle>,
    session: &Session,
    label: &CStr,
    filename: &CStr,
    cook: bool,
) -> Result<raw::HAPI_NodeId> {
    unsafe {
        let mut handle = uninit!();
        raw::HAPI_LoadNodeFromFile(
            session.ptr(),
            filename.as_ptr(),
            parent_node.map(|n| n.0).unwrap_or(-1),
            label.as_ptr(),
            cook as i8,
            handle.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        Ok(handle.assume_init())
    }
}

pub fn set_geo_vertex_list(node: &HoudiniNode, part_id: i32, list: &[i32]) -> Result<()> {
    unsafe {
        raw::HAPI_SetVertexList(
            node.session.ptr(),
            node.handle.0,
            part_id,
            list.as_ptr(),
            0,
            list.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn get_geo_vertex_list(
    session: &Session,
    node: NodeHandle,
    part_id: i32,
    start: i32,
    length: i32,
) -> Result<Vec<i32>> {
    unsafe {
        let mut array = vec![0; length as usize];
        raw::HAPI_GetVertexList(
            session.ptr(),
            node.0,
            part_id,
            array.as_mut_ptr(),
            start,
            length,
        )
        .check_err(Some(session))?;
        Ok(array)
    }
}

pub fn set_geo_face_counts(node: &HoudiniNode, part_id: i32, list: &[i32]) -> Result<()> {
    unsafe {
        raw::HAPI_SetFaceCounts(
            node.session.ptr(),
            node.handle.0,
            part_id,
            list.as_ptr(),
            0,
            list.len() as i32,
        )
        .check_err(Some(&node.session))
    }
}

pub fn get_viewport(session: &Session) -> Result<raw::HAPI_Viewport> {
    let mut vp = uninit!();
    unsafe {
        raw::HAPI_GetViewport(session.ptr(), vp.as_mut_ptr()).check_err(Some(session))?;
        Ok(vp.assume_init())
    }
}

pub fn set_viewport(session: &Session, viewport: &Viewport) -> Result<()> {
    unsafe { raw::HAPI_SetViewport(session.ptr(), viewport.ptr()).check_err(Some(session)) }
}

pub fn set_session_sync(session: &Session, enable: bool) -> Result<()> {
    unsafe { raw::HAPI_SetSessionSync(session.ptr(), enable as i8).check_err(Some(session)) }
}

pub fn set_node_display(session: &Session, node: NodeHandle, on: bool) -> Result<()> {
    unsafe { raw::HAPI_SetNodeDisplay(session.ptr(), node.0, on as i32).check_err(Some(session)) }
}

pub fn get_object_info(session: &Session, node: NodeHandle) -> Result<raw::HAPI_ObjectInfo> {
    let mut info = uninit!();
    unsafe {
        raw::HAPI_GetObjectInfo(session.ptr(), node.0, info.as_mut_ptr())
            .check_err(Some(session))?;

        Ok(info.assume_init())
    }
}

pub fn get_object_transform(
    session: &Session,
    node: NodeHandle,
    relative: Option<NodeHandle>,
    rst: raw::RSTOrder,
) -> Result<raw::HAPI_Transform> {
    let mut t = uninit!();
    unsafe {
        raw::HAPI_GetObjectTransform(
            session.ptr(),
            node.0,
            relative.map(|n| n.0).unwrap_or(-1),
            rst,
            t.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        Ok(t.assume_init())
    }
}

pub fn set_object_transform(
    session: &Session,
    node: NodeHandle,
    transform: &raw::HAPI_TransformEuler,
) -> Result<()> {
    unsafe {
        raw::HAPI_SetObjectTransform(session.ptr(), node.0, transform as *const _)
            .check_err(Some(session))
    }
}

pub fn set_session_sync_info(session: &Session, info: &raw::HAPI_SessionSyncInfo) -> Result<()> {
    unsafe {
        raw::HAPI_SetSessionSyncInfo(session.ptr(), info as *const _).check_err(Some(session))
    }
}

pub fn get_session_sync_info(session: &Session) -> Result<raw::HAPI_SessionSyncInfo> {
    let mut info = uninit!();
    unsafe {
        raw::HAPI_GetSessionSyncInfo(session.ptr(), info.as_mut_ptr()).check_err(Some(session))?;
        Ok(info.assume_init())
    }
}

pub fn set_parm_anim_curve(
    session: &Session,
    node: NodeHandle,
    parm: ParmHandle,
    index: i32,
    keys: &[raw::HAPI_Keyframe],
) -> Result<()> {
    unsafe {
        raw::HAPI_SetAnimCurve(
            session.ptr(),
            node.0,
            parm.0,
            index,
            keys.as_ptr(),
            keys.len() as i32,
        )
        .check_err(Some(session))
    }
}

pub fn set_transform_anim_curve(
    session: &Session,
    node: NodeHandle,
    comp: raw::TransformComponent,
    keys: &[raw::HAPI_Keyframe],
) -> Result<()> {
    unsafe {
        raw::HAPI_SetTransformAnimCurve(
            session.ptr(),
            node.0,
            comp,
            keys.as_ptr(),
            keys.len() as i32,
        )
        .check_err(Some(session))
    }
}

pub fn save_geo_to_memory(session: &Session, node: NodeHandle, format: &CStr) -> Result<Vec<i8>> {
    unsafe {
        let mut size = uninit!();
        raw::HAPI_GetGeoSize(session.ptr(), node.0, format.as_ptr(), size.as_mut_ptr())
            .check_err(Some(session))?;
        let size = size.assume_init();
        let mut buffer = Vec::new();
        buffer.resize(size as usize, 0);
        raw::HAPI_SaveGeoToMemory(session.ptr(), node.0, buffer.as_mut_ptr(), size)
            .check_err(Some(session))?;
        Ok(buffer)
    }
}

pub fn load_geo_from_memory(
    session: &Session,
    node: NodeHandle,
    data: &[i8],
    format: &CStr,
) -> Result<()> {
    unsafe {
        raw::HAPI_LoadGeoFromMemory(
            session.ptr(),
            node.0,
            format.as_ptr(),
            data.as_ptr(),
            data.len() as i32,
        )
        .check_err(Some(session))
    }
}

pub fn commit_geo(node: &HoudiniNode) -> Result<()> {
    unsafe { raw::HAPI_CommitGeo(node.session.ptr(), node.handle.0).check_err(Some(&node.session)) }
}

pub fn revert_geo(node: &HoudiniNode) -> Result<()> {
    unsafe { raw::HAPI_RevertGeo(node.session.ptr(), node.handle.0).check_err(Some(&node.session)) }
}

pub fn session_get_license_type(session: &Session) -> Result<raw::License> {
    unsafe {
        let mut ret = uninit!();
        raw::HAPI_GetSessionEnvInt(
            session.ptr(),
            raw::SessionEnvIntType::License,
            ret.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        // SAFETY: License enum is repr i32
        Ok(std::mem::transmute(ret.assume_init()))
    }
}

pub fn get_environment_int(_type: raw::EnvIntType) -> Result<i32> {
    unsafe {
        let mut ret = uninit!();
        raw::HAPI_GetEnvInt(_type, ret.as_mut_ptr()).error_message("get_environment_int")?;
        Ok(ret.assume_init())
    }
}

pub fn get_preset(
    session: &Session,
    node: NodeHandle,
    name: &CStr,
    _type: raw::PresetType,
) -> Result<Vec<i8>> {
    unsafe {
        let mut length = uninit!();
        raw::HAPI_GetPresetBufLength(
            session.ptr(),
            node.0,
            _type,
            name.as_ptr(),
            length.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        let mut buffer = Vec::new();
        buffer.resize(length.assume_init() as usize, 0);
        raw::HAPI_GetPreset(
            session.ptr(),
            node.0,
            buffer.as_mut_ptr(),
            buffer.len() as i32,
        )
        .check_err(Some(session))?;
        Ok(buffer)
    }
}

pub fn set_preset(
    session: &Session,
    node: NodeHandle,
    name: &CStr,
    _type: raw::PresetType,
    data: &[i8],
) -> Result<()> {
    unsafe {
        raw::HAPI_SetPreset(
            session.ptr(),
            node.0,
            _type,
            name.as_ptr(),
            data.as_ptr(),
            data.len() as i32,
        )
        .check_err(Some(session))
    }
}

pub fn get_material_info(session: &Session, node: NodeHandle) -> Result<raw::HAPI_MaterialInfo> {
    unsafe {
        let mut mat = uninit!();
        raw::HAPI_GetMaterialInfo(session.ptr(), node.0, mat.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(mat.assume_init())
    }
}

pub fn get_material_node_ids_on_faces(
    session: &Session,
    node: NodeHandle,
    face_count: i32,
    part_id: i32,
) -> Result<(bool, Vec<i32>)> {
    unsafe {
        let mut are_all_the_same = uninit!();
        let mut ids = vec![0; face_count as usize];
        raw::HAPI_GetMaterialNodeIdsOnFaces(
            session.ptr(),
            node.0,
            part_id,
            are_all_the_same.as_mut_ptr(),
            ids.as_mut_ptr(),
            0,
            face_count,
        )
        .check_err(Some(session))?;
        Ok((are_all_the_same.assume_init() > 0, ids))
    }
}

pub fn get_instanced_part_ids(
    session: &Session,
    node: NodeHandle,
    part_id: i32,
    count: i32,
) -> Result<Vec<i32>> {
    unsafe {
        let mut parts = vec![0; count as usize];
        raw::HAPI_GetInstancedPartIds(session.ptr(), node.0, part_id, parts.as_mut_ptr(), 0, count)
            .check_err(Some(session))?;
        Ok(parts)
    }
}

pub fn get_group_count_on_instance_part(
    session: &Session,
    node: NodeHandle,
    part_id: i32,
) -> Result<(i32, i32)> {
    unsafe {
        let (mut point, mut prim) = (uninit!(), uninit!());
        raw::HAPI_GetGroupCountOnPackedInstancePart(
            session.ptr(),
            node.0,
            part_id,
            point.as_mut_ptr(),
            prim.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        Ok((point.assume_init(), prim.assume_init()))
    }
}

pub fn get_group_names_on_instance_part(
    session: &Session,
    node: NodeHandle,
    part_id: i32,
    group: raw::GroupType,
) -> Result<StringArray> {
    unsafe {
        let count = get_group_count_on_instance_part(session, node, part_id)?;
        let count = match group {
            raw::GroupType::Point => count.0,
            raw::GroupType::Prim => count.1,
            _ => unreachable!("Unsupported GroupType"),
        };
        let mut handles = vec![0; count as usize];
        raw::HAPI_GetGroupNamesOnPackedInstancePart(
            session.ptr(),
            node.0,
            part_id,
            group,
            handles.as_mut_ptr(),
            count,
        )
        .check_err(Some(session))?;
        crate::stringhandle::get_string_array(&handles, session)
    }
}

pub fn get_instanced_part_transforms(
    session: &Session,
    node: NodeHandle,
    part_id: i32,
    order: raw::RSTOrder,
    count: i32,
) -> Result<Vec<raw::HAPI_Transform>> {
    unsafe {
        let mut transforms = vec![raw::HAPI_Transform_Create(); count as usize];
        raw::HAPI_GetInstancerPartTransforms(
            session.ptr(),
            node.0,
            part_id,
            order,
            transforms.as_mut_ptr(),
            0,
            count,
        )
        .check_err(Some(session))?;
        Ok(transforms)
    }
}

pub fn convert_transform(
    session: &Session,
    tr_in: &raw::HAPI_TransformEuler,
    rst_order: raw::RSTOrder,
    rot_order: raw::XYZOrder,
) -> Result<raw::HAPI_TransformEuler> {
    unsafe {
        let mut out = raw::HAPI_TransformEuler_Create();
        raw::HAPI_ConvertTransform(
            session.ptr(),
            tr_in,
            rst_order,
            rot_order,
            &mut out as *mut _,
        )
        .check_err(Some(session))?;
        Ok(out)
    }
}

pub fn convert_matrix_to_euler(
    session: &Session,
    matrix: &[f32; 16],
    rst_order: raw::RSTOrder,
    rot_order: raw::XYZOrder,
) -> Result<raw::HAPI_TransformEuler> {
    unsafe {
        let mut out = raw::HAPI_TransformEuler_Create();
        raw::HAPI_ConvertMatrixToEuler(
            session.ptr(),
            matrix.as_ptr(),
            rst_order,
            rot_order,
            &mut out as *mut _,
        )
        .check_err(Some(session))?;
        Ok(out)
    }
}

pub fn convert_matrix_to_quat(
    session: &Session,
    matrix: &[f32; 16],
    rst_order: raw::RSTOrder,
) -> Result<raw::HAPI_Transform> {
    unsafe {
        let mut out = raw::HAPI_Transform_Create();
        raw::HAPI_ConvertMatrixToQuat(
            session.ptr(),
            matrix.as_ptr(),
            rst_order,
            &mut out as *mut _,
        )
        .check_err(Some(session))?;
        Ok(out)
    }
}

pub fn convert_transform_euler_to_matrix(
    session: &Session,
    tr: &raw::HAPI_TransformEuler,
) -> Result<[f32; 16]> {
    unsafe {
        let mut out = [0.0; 16];
        raw::HAPI_ConvertTransformEulerToMatrix(session.ptr(), tr as *const _, &mut out as *mut _)
            .check_err(Some(session))?;
        Ok(out)
    }
}

pub fn convert_transform_quat_to_matrix(
    session: &Session,
    tr: &raw::HAPI_Transform,
) -> Result<[f32; 16]> {
    unsafe {
        let mut out = [0.0; 16];
        raw::HAPI_ConvertTransformQuatToMatrix(session.ptr(), tr as *const _, &mut out as *mut _)
            .check_err(Some(session))?;
        Ok(out)
    }
}

pub fn get_supported_image_file_formats(
    session: &Session,
) -> Result<Vec<raw::HAPI_ImageFileFormat>> {
    unsafe {
        let mut count = uninit!();
        raw::HAPI_GetSupportedImageFileFormatCount(session.ptr(), count.as_mut_ptr())
            .check_err(Some(session))?;
        let count = count.assume_init();
        let mut array = vec![raw::HAPI_ImageFileFormat_Create(); count as usize];
        raw::HAPI_GetSupportedImageFileFormats(session.ptr(), array.as_mut_ptr(), count)
            .check_err(Some(session))?;
        Ok(array)
    }
}

pub fn render_texture_to_image(
    session: &Session,
    material: NodeHandle,
    parm: ParmHandle,
) -> Result<()> {
    unsafe {
        raw::HAPI_RenderTextureToImage(session.ptr(), material.0, parm.0).check_err(Some(session))
    }
}

pub fn set_image_info(session: &Session, material: NodeHandle, info: &ImageInfo) -> Result<()> {
    unsafe {
        raw::HAPI_SetImageInfo(session.ptr(), material.0, info.ptr()).check_err(Some(session))
    }
}

pub fn get_image_info(session: &Session, material: NodeHandle) -> Result<raw::HAPI_ImageInfo> {
    unsafe {
        let mut info = uninit!();
        raw::HAPI_GetImageInfo(session.ptr(), material.0, info.as_mut_ptr())
            .check_err(Some(session))?;
        Ok(info.assume_init())
    }
}

pub fn extract_image_to_file(
    session: &Session,
    material: NodeHandle,
    file_format: &CStr,
    image_planes: &CStr,
    dest_folder: &CStr,
    dest_file: &CStr,
) -> Result<String> {
    let mut handle = uninit!();
    unsafe {
        raw::HAPI_ExtractImageToFile(
            session.ptr(),
            material.0,
            file_format.as_ptr(),
            image_planes.as_ptr(),
            dest_folder.as_ptr(),
            dest_file.as_ptr(),
            handle.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        crate::stringhandle::get_string(handle.assume_init(), session)
    }
}

pub fn extract_image_to_memory(
    session: &Session,
    material: NodeHandle,
    file_format: &CStr,
    image_planes: &CStr,
) -> Result<Vec<i8>> {
    unsafe {
        let mut size = uninit!();
        raw::HAPI_ExtractImageToMemory(
            session.ptr(),
            material.0,
            file_format.as_ptr(),
            image_planes.as_ptr(),
            size.as_mut_ptr(),
        )
        .check_err(Some(session))?;
        let size = size.assume_init() as usize;
        let mut buffer = Vec::with_capacity(size);
        buffer.resize(size, 0);
        raw::HAPI_GetImageMemoryBuffer(
            session.ptr(),
            material.0,
            buffer.as_mut_ptr(),
            buffer.len() as i32,
        )
        .check_err(Some(session))?;
        Ok(buffer)
    }
}

pub fn get_image_planes(session: &Session, material: NodeHandle) -> Result<StringArray> {
    unsafe {
        let mut count = uninit!();
        raw::HAPI_GetImagePlaneCount(session.ptr(), material.0, count.as_mut_ptr())
            .check_err(Some(session))?;
        let count = count.assume_init();
        let mut handles = vec![0; count as usize];
        raw::HAPI_GetImagePlanes(session.ptr(), material.0, handles.as_mut_ptr(), count)
            .check_err(Some(session))?;
        crate::stringhandle::get_string_array(&handles, session)
    }
}
