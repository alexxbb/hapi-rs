use crate::errors::{HapiError, Kind, Result};
use crate::{session::Session};
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
        let length = crate::ffi::get_string_buff_len(session, handle)?;
        let buffer = crate::ffi::get_string(session, handle, length)?;
        Ok(buffer)
    }
}

pub fn get_string_batch(handles: &[i32], session: &Session) -> Result<Vec<String>> {
    unsafe {
        let length = crate::ffi::get_string_batch_size(handles, session)?;
        if length > 0 {
            let buffer = crate::ffi::get_string_batch(length, session)?;
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
