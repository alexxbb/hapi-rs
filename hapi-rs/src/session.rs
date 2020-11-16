use super::errors::*;
use crate::cookoptions::CookOptions;
use crate::ffi;
use std::mem::MaybeUninit;
use std::sync::Arc;

#[derive(Debug)]
pub struct SessionHandle {
    inner: ffi::HAPI_Session,
}

#[derive(Debug, Clone)]
pub struct Session {
    handle: Arc<SessionHandle>,
}

impl SessionHandle {
    #[inline]
    pub fn ffi_ptr(&self) -> *const ffi::HAPI_Session {
        &self.inner as *const _
    }
}

impl Session {
    pub fn handle(&self) -> &Arc<SessionHandle> {
        &self.handle
    }

    pub fn ffi_ptr(&self) -> *const ffi::HAPI_Session {
        self.handle.ffi_ptr()
    }

    pub fn new_in_process() -> Result<Session> {
        let mut ses = MaybeUninit::uninit();
        unsafe {
            match ffi::HAPI_CreateInProcessSession(ses.as_mut_ptr()) {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(Session {
                    handle: Arc::new(SessionHandle {
                        inner: ses.assume_init(),
                    }),
                }),
                e => hapi_err!(e),
            }
        }
    }
    pub fn initialize(&self) -> Result<()> {
        let co = CookOptions::default();
        use std::ptr::null;
        unsafe {
            let result = ffi::HAPI_Initialize(
                self.handle.ffi_ptr(),
                co.const_ptr(),
                1,
                -1,
                null(),
                null(),
                null(),
                null(),
                null(),
            );
            hapi_ok!(result, self.handle.ffi_ptr())
        }
    }
}

impl Drop for SessionHandle {
    fn drop(&mut self) {
        eprintln!("Dropping last SessionHandle");
        eprintln!("HAPI_Cleanup");
        unsafe {
            use ffi::HAPI_Result::*;
            if !matches!(ffi::HAPI_Cleanup(self.ffi_ptr()), HAPI_RESULT_SUCCESS) {
                eprintln!("Dropping SessionHandle failed!");
            }
        }
    }
}
