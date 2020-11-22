use crate::errors::{HapiError, Kind, Result};
use crate::ffi::{
    HAPI_GetString, HAPI_GetStringBufLength, HAPI_IsSessionValid, HAPI_Result, HAPI_Session,
    HAPI_StringHandle,
};
use std::mem::MaybeUninit;
use std::os::raw::c_char;

pub fn get_string(handle: HAPI_StringHandle, session: *const HAPI_Session) -> Result<String> {
    unsafe {
        let mut length = MaybeUninit::uninit();
        match HAPI_GetStringBufLength(session, handle, length.as_mut_ptr()) {
            HAPI_Result::HAPI_RESULT_SUCCESS => {
                let length = length.assume_init();
                let mut buffer = vec![0u8; length as usize];
                let ptr = buffer.as_mut_ptr() as *mut c_char;
                match HAPI_GetString(session, handle, ptr, length) {
                    HAPI_Result::HAPI_RESULT_SUCCESS => {
                        buffer.truncate(length as usize - 1);
                        Ok(String::from_utf8_unchecked(buffer))
                    }
                    e => Err(HapiError::new(
                        Kind::Hapi(e),
                        Some(session),
                        Some("Retrieving error string failed"),
                    )),
                }
            }
            e => Err(HapiError::new(
                Kind::Hapi(e),
                Some(session),
                Some("Retrieving string length failed"),
            )),
        }
    }
}
