use crate::ffi;
use std::mem::MaybeUninit;
use crate::errors::HAPI_Error;

pub type Result<T> = std::result::Result<T, HAPI_Error>;

pub struct Session {
    inner: ffi::HAPI_Session
}

impl Session {
    pub fn new_in_process() -> Result<Session> {
        let mut s = MaybeUninit::uninit();
        unsafe {
            match ffi::HAPI_CreateInProcessSession(s.as_mut_ptr()) {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => {
                    Ok(Session { inner: s.assume_init() })
                }
                e => Err(e.into())
            }
        }
    }
}

