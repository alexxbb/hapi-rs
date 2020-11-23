use crate::errors::{HapiError, Kind, Result};
use crate::{ffi, session::Session};
use std::mem::MaybeUninit;
use std::os::raw::c_char;

pub fn get_string(handle: ffi::HAPI_StringHandle, session: &Session) -> Result<String> {
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
