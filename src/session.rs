use crate::ffi;
use std::mem::MaybeUninit;
use crate::errors::HAPI_Error;

type Result<T> = std::result::Result<T, HAPI_Error>;

struct Session {
    inner: ffi::HAPI_Session
}

impl Session {
    fn new_in_process() -> Result<Session>{
        let mut s = MaybeUninit::uninit();
        unsafe {
            let res = ffi::HAPI_CreateInProcessSession(s.as_mut_ptr());
            Session {inner: s.assume_init()}
        }
    }
}

