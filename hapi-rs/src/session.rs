use crate::auto::rusty::SessionType;
use crate::ffi;
use super::errors::*;
use crate::cookoptions::CookOptions;
use std::mem::MaybeUninit;

#[derive(Debug)]
pub struct Session {
    pub(crate) inner: ffi::HAPI_Session,
}

impl Session {
    pub fn const_ptr(&self) -> *const ffi::HAPI_Session {
        &self.inner as *const _
    }
    pub fn session_type(&self) -> SessionType {
        self.inner.type_.into()
    }

    pub fn ffi_ptr(&self) -> *const ffi::HAPI_Session {
        &self.inner as *const _
    }

    pub fn new_in_process() -> Result<Session> {
        let mut ses = MaybeUninit::uninit();
        unsafe {
            match ffi::HAPI_CreateInProcessSession(ses.as_mut_ptr()) {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(Session {
                    inner: ses.assume_init(),
                }),
                e => hapi_err!(e)
            }
        }
    }
    pub fn initialize(&self) -> Result<()> {
        let co = CookOptions::default();
        use std::ptr::null;
        unsafe {
            let result = ffi::HAPI_Initialize(
                &self.inner as *const _,
                co.const_ptr(),
                0,
                -1,
                null(),
                null(),
                null(),
                null(),
                null(),
            );
            hapi_ok!(result, &self.inner as *const _)
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        eprintln!("Dropping session");
        unsafe {
            use ffi::HAPI_Result::*;
            if !matches!(
                ffi::HAPI_Cleanup(&self.inner as *const _),
                HAPI_RESULT_SUCCESS
            ) {
                eprintln!("Dropping session failed!");
            }
        }
    }
}
