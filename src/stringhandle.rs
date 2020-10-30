use crate::errors::{HAPI_Error, Result};
use crate::ffi::{
    HAPI_GetString, HAPI_GetStringBufLength, HAPI_Result, HAPI_Session, HAPI_StringHandle,
};
use std::ffi::CString;
use std::mem::MaybeUninit;

pub fn get_string(handle: HAPI_StringHandle, session: *const HAPI_Session) -> Result<String> {
    unsafe {
        let mut length = MaybeUninit::uninit();
        match HAPI_GetStringBufLength(session, handle, length.as_mut_ptr()) {
            HAPI_Result::HAPI_RESULT_SUCCESS => {
                let length = length.assume_init();
                let buffer = Vec::<u8>::with_capacity(length as usize);
                let buffer = CString::from_vec_unchecked(buffer);
                let ptr = buffer.into_raw();
                match HAPI_GetString(session, handle, ptr, length) {
                    HAPI_Result::HAPI_RESULT_SUCCESS => {
                        let cstr = CString::from_raw(ptr);
                        Ok(cstr.to_string_lossy().to_string())
                    }
                    e => Err(HAPI_Error::new(e, session)),
                }
            }
            e => Err(HAPI_Error::new(e, session)),
        }
    }
}
