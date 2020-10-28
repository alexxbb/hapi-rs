use crate::ffi;
use std::mem::MaybeUninit;
use std::ptr::null;

use crate::errors::HAPI_Error;


pub fn get_last_error(session: Option<*const ffi::HAPI_Session>) -> Result<String, HAPI_Error> {
    use ffi::HAPI_StatusType::HAPI_STATUS_CALL_RESULT;
    use ffi::HAPI_StatusVerbosity::HAPI_STATUSVERBOSITY_0;
    unsafe {
        let mut length = MaybeUninit::uninit();
        let res = ffi::HAPI_GetStatusStringBufLength(
            session.unwrap_or(null()),
            HAPI_STATUS_CALL_RESULT,
            HAPI_STATUSVERBOSITY_0,
            length.as_mut_ptr(),
        );
        match res {
            ffi::HAPI_Result::HAPI_RESULT_SUCCESS => {
                let length = length.assume_init();
                let mut buf = vec![0u8; length as usize];
                match ffi::HAPI_GetStatusString(
                    session.unwrap_or(null()), HAPI_STATUS_CALL_RESULT,
                    // SAFETY: casting to u8 to i8 (char)?
                    buf.as_mut_ptr() as *mut i8, length) {
                    ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(String::from_utf8_unchecked(buf)),
                    e => Err(e.into())
                }
            }
            e => Err(e.into())
        }
    }
}

