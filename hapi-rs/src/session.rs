use super::errors::*;
use crate::asset::AssetLibrary;
use crate::cookoptions::CookOptions;
use crate::ffi;
use crate::node::{HoudiniNode, NodeType};
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SessionHandle {
    inner: Arc<ffi::HAPI_Session>,
}

#[derive(Debug)]
pub struct Session {
    handle: SessionHandle,
}

impl SessionHandle {
    #[inline]
    pub fn ptr(&self) -> *const ffi::HAPI_Session {
        self.inner.as_ref() as *const _
    }
}

// TODO: split session into SessionSync and SessionAsync
impl Session {
    pub fn handle(&self) -> &SessionHandle {
        &self.handle
    }

    pub fn new_in_process() -> Result<Session> {
        let mut ses = MaybeUninit::uninit();
        unsafe {
            match ffi::HAPI_CreateInProcessSession(ses.as_mut_ptr()) {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(Session {
                    handle: SessionHandle {
                        inner: Arc::new(ses.assume_init()),
                    },
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
                self.handle.ptr(),
                co.const_ptr(),
                1,
                -1,
                null(),
                null(),
                null(),
                null(),
                null(),
            );
            hapi_ok!(result, self.handle.ptr())
        }
    }

    pub fn create_node<T: Into<Vec<u8>>>(
        &self,
        name: T,
        label: Option<T>,
        parent: Option<HoudiniNode>,
    ) -> Result<HoudiniNode> {
        HoudiniNode::create_sync(name, label, parent, self.handle.clone(), false)
    }

    pub fn save_hip(&self, name: &str) -> Result<()> {
        unsafe {
            let name = CString::from_vec_unchecked(name.into());
            ffi::HAPI_SaveHIPFile(self.handle.ptr(), name.as_ptr(), 0).result(self.handle.ptr())
        }
    }

    pub fn load_hip(&self, name: &str, cook: bool) -> Result<()> {
        unsafe {
            let name = CString::from_vec_unchecked(name.into());
            ffi::HAPI_LoadHIPFile(self.handle.ptr(), name.as_ptr(), cook as i8)
                .result(self.handle.ptr())
        }
    }

    pub fn merge_hip(&self, name: &str, cook: bool) -> Result<i32> {
        unsafe {
            let name = CString::from_vec_unchecked(name.into());
            let mut id = MaybeUninit::uninit();
            ffi::HAPI_MergeHIPFile(
                self.handle.ptr(),
                name.as_ptr(),
                cook as i8,
                id.as_mut_ptr(),
            )
            .result(self.handle.ptr())?;
            Ok(id.assume_init())
        }
    }

    pub fn load_asset_file(&self, file: &str) -> Result<AssetLibrary> {
        AssetLibrary::from_file(file, self.handle.clone())
    }

    pub fn interrupt(&self) -> Result<()> {
        unsafe { ffi::HAPI_Interrupt(self.handle.ptr()).result(self.handle.ptr()) }
    }
}

impl Drop for SessionHandle {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            eprintln!("Dropping last SessionHandle");
            eprintln!("HAPI_Cleanup");
            unsafe {
                use ffi::HAPI_Result::*;
                if !matches!(ffi::HAPI_Cleanup(self.ptr()), HAPI_RESULT_SUCCESS) {
                    eprintln!("Dropping SessionHandle failed!");
                }
            }
        }
    }
}
