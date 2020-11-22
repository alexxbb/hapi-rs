use crate::{
    asset::AssetLibrary,
    auto::rusty::{State, StatusType},
    cookoptions::CookOptions,
    errors::*,
    ffi,
    node::{HoudiniNode, NodeType},
};
#[rustfmt::skip]
use std::{
    ffi::CString,
    mem::MaybeUninit,
    ops::Deref,
    ptr::null,
    sync::Arc,
};

#[derive(Debug, Clone)]
pub struct Session {
    handle: Arc<ffi::HAPI_Session>,
    pub unsync: bool,
}

impl Session {
    #[inline]
    pub fn ptr(&self) -> *const ffi::HAPI_Session {
        self.handle.as_ref() as *const _
    }
    pub fn new_in_process(unsync: bool) -> Result<Session> {
        let mut ses = MaybeUninit::uninit();
        unsafe {
            match ffi::HAPI_CreateInProcessSession(ses.as_mut_ptr()) {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(Session {
                    handle: Arc::new(ses.assume_init()),
                    unsync,
                }),
                e => hapi_err!(e),
            }
        }
    }

    pub fn start_named_pipe_server(path: &str) -> Result<i32> {
        let pid = unsafe {
            let mut pid = MaybeUninit::uninit();
            let cs = CString::new(path)?;
            let opts = ffi::HAPI_ThriftServerOptions {
                autoClose: 1,
                timeoutMs: 1000.0,
            };
            ffi::HAPI_StartThriftNamedPipeServer(&opts as *const _, cs.as_ptr(), pid.as_mut_ptr())
                .result(null(), Some("Could not start thrift server"))?;
            pid.assume_init()
        };
        Ok(pid)
    }

    pub fn new_named_pipe(path: &str, unsync: bool) -> Result<Session> {
        let session = unsafe {
            let mut handle = MaybeUninit::uninit();
            let cs = CString::new(path)?;
            ffi::HAPI_CreateThriftNamedPipeSession(handle.as_mut_ptr(), cs.as_ptr())
                .result(null(), Some("Could not start piped session"))?;
            handle.assume_init()
        };
        Ok(Session {
            handle: Arc::new(session),
            unsync,
        })
    }

    pub fn initialize(&self) -> Result<()> {
        let co = CookOptions::default();
        use std::ptr::null;
        unsafe {
            let result = ffi::HAPI_Initialize(
                self.ptr(),
                co.const_ptr(),
                self.unsync as i8,
                -1,
                null(),
                null(),
                null(),
                null(),
                null(),
            );
            hapi_ok!(result, self.ptr(), None)
        }
    }

    pub fn create_node_blocking(
        &self,
        name: &str,
        label: Option<&str>,
        parent: Option<HoudiniNode>,
    ) -> Result<HoudiniNode> {
        HoudiniNode::create_blocking(name, label, parent, self.clone(), false)
    }

    pub fn save_hip(&self, name: &str) -> Result<()> {
        unsafe {
            let name = CString::new(name)?;
            ffi::HAPI_SaveHIPFile(self.ptr(), name.as_ptr(), 0)
                .result(self.ptr(), None)
        }
    }

    pub fn load_hip(&self, name: &str, cook: bool) -> Result<()> {
        unsafe {
            let name = CString::new(name)?;
            ffi::HAPI_LoadHIPFile(self.ptr(), name.as_ptr(), cook as i8)
                .result(self.ptr(), None)
        }
    }

    pub fn merge_hip(&self, name: &str, cook: bool) -> Result<i32> {
        unsafe {
            let name = CString::new(name)?;
            let mut id = MaybeUninit::uninit();
            ffi::HAPI_MergeHIPFile(
                self.ptr(),
                name.as_ptr(),
                cook as i8,
                id.as_mut_ptr(),
            )
            .result(self.ptr(), None)?;
            Ok(id.assume_init())
        }
    }

    pub fn load_asset_file(&self, file: &str) -> Result<AssetLibrary> {
        AssetLibrary::from_file(file, self.clone())
    }

    pub fn interrupt(&self) -> Result<()> {
        unsafe { ffi::HAPI_Interrupt(self.ptr()).result(self.ptr(), None) }
    }

    pub fn get_status(&self, flag: StatusType) -> Result<State> {
        let status = unsafe {
            let mut status = MaybeUninit::uninit();
            ffi::HAPI_GetStatus(self.ptr(), flag.into(), status.as_mut_ptr())
                .result(self.ptr(), None)?;
            status.assume_init()
        };
        Ok(State::from(status))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if Arc::strong_count(&self.handle) == 1 {
            eprintln!("Dropping last Session");
            eprintln!("HAPI_Cleanup");
            unsafe {
                use ffi::HAPI_Result::*;
                if !matches!(ffi::HAPI_Cleanup(self.ptr()), HAPI_RESULT_SUCCESS) {
                    eprintln!("HAPI_Cleanup failed!");
                }
                if !matches!(ffi::HAPI_CloseSession(self.ptr()), HAPI_RESULT_SUCCESS) {
                    eprintln!("HAPI_CloseSession failed!");
                }
            }
        }
    }
}
