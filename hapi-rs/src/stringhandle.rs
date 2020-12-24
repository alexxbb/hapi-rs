use crate::errors::{HapiError, Kind, Result};
use crate::{ffi, session::Session};
use std::io::BufRead;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::ffi::CString;

pub fn get_string(handle: i32, session: &Session) -> Result<String> {
    unsafe {
        let mut bytes = get_string_bytes(handle, session)?;
        Ok(String::from_utf8_unchecked(bytes))
    }
}

pub fn get_cstring(handle: i32, session: &Session) -> Result<CString> {
    unsafe {
        let mut bytes = get_string_bytes(handle, session)?;
        Ok(CString::from_vec_unchecked(bytes))
    }
}

pub(crate) fn get_string_bytes(handle: i32, session: &Session) -> Result<Vec<u8>> {
    unsafe {
        let mut length = MaybeUninit::uninit();
        ffi::HAPI_GetStringBufLength(session.ptr(), handle, length.as_mut_ptr())
            .result_with_session(|| session.clone())?;
        let length = length.assume_init();
        let mut buffer = vec![0u8; length as usize];
        let ptr = buffer.as_mut_ptr() as *mut c_char;
        ffi::HAPI_GetString(session.ptr(), handle, ptr, length)
            .result_with_message("get_string failed")?;
        buffer.truncate(length as usize - 1);
        Ok(buffer)
    }
}

pub fn get_string_batch(handles: &[i32], session: &Session) -> Result<Vec<String>> {
    unsafe {
        let mut length = MaybeUninit::uninit();
        ffi::HAPI_GetStringBatchSize(
            session.ptr(),
            handles.as_ptr(),
            handles.len() as i32,
            length.as_mut_ptr(),
        )
        .result_with_session(|| session.clone())?;
        let length = length.assume_init();
        if length > 0 {
            let mut buffer = vec![0u8; length as usize];
            ffi::HAPI_GetStringBatch(session.ptr(), buffer.as_mut_ptr() as *mut _, length)
                .result_with_session(|| session.clone())?;
            buffer.truncate(length as usize - 1);
            let mut buffer = std::mem::ManuallyDrop::new(buffer);
            let strings = buffer
                .split_mut(|b| *b == b'\0')
                .map(|s| String::from_utf8_lossy(s).to_string())
                .collect::<Vec<String>>();
            Ok(strings)
        } else {
            Ok(vec![])
        }
    }
}
