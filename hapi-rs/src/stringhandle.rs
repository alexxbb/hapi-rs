use crate::errors::{HapiError, Kind, Result};
use crate::{ffi, session::Session};
use std::io::BufRead;
use std::mem::MaybeUninit;
use std::os::raw::c_char;

pub fn get_string(handle: i32, session: &Session) -> Result<String> {
    unsafe {
        let mut length = MaybeUninit::uninit();
        ffi::HAPI_GetStringBufLength(session.ptr(), handle, length.as_mut_ptr())
            .result_with_session(|| session.clone())?;
        let length = length.assume_init();
        let mut buffer = vec![0u8; length as usize];
        let ptr = buffer.as_mut_ptr() as *mut c_char;
        ffi::HAPI_GetString(session.ptr(), handle, ptr, length).result_with_message(None)?;
        buffer.truncate(length as usize - 1);
        Ok(String::from_utf8_unchecked(buffer))
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
                .map(|s| String::from_raw_parts(s.as_mut_ptr(), s.len(), s.len()))
                .collect::<Vec<String>>();
            Ok(strings)
        } else {
            Ok(vec![])
        }
    }
}
