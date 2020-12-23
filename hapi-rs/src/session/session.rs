use crate::{
    asset::AssetLibrary,
    session::CookOptions,
    errors::*,
    ffi,
    node::{HoudiniNode, NodeHandle},
};

use crate::ffi:: {
    StatusType, State
};
pub use crate::ffi:: {
    StatusVerbosity
};

#[rustfmt::skip]
use std::{
    ffi::CString,
    mem::MaybeUninit,
    ops::Deref,
    ptr::null,
    sync::Arc,
    path::Path
};

use log::{debug, error, warn};
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub enum CookResult {
    Succeeded,
    Warnings,
    Errored(String),
}

#[derive(Debug, Clone)]
pub struct Session {
    handle: Arc<ffi::HAPI_Session>,
    pub unsync: bool,
    cleanup: bool,
}

impl Session {
    #[inline]
    pub fn ptr(&self) -> *const ffi::HAPI_Session {
        self.handle.as_ref() as *const _
    }
    pub fn new_in_process() -> Result<Session> {
        debug!("Creating new in-process session");
        let mut ses = MaybeUninit::uninit();
        unsafe {
            ffi::HAPI_CreateInProcessSession(ses.as_mut_ptr())
                .result_with_message("Session::new_in_process failed")?;
            Ok(Session {
                handle: Arc::new(ses.assume_init()),
                unsync: false,
                cleanup: true,
            })
        }
    }

    pub fn start_named_pipe_server(path: &str) -> Result<i32> {
        debug!("Starting named pipe server: {}", path);
        let pid = unsafe {
            let mut pid = MaybeUninit::uninit();
            let cs = CString::new(path)?;
            let opts = ffi::HAPI_ThriftServerOptions {
                autoClose: 1,
                timeoutMs: 1000.0,
            };
            ffi::HAPI_StartThriftNamedPipeServer(&opts as *const _, cs.as_ptr(), pid.as_mut_ptr())
                .result_with_message("Could not start thrift server")?;
            pid.assume_init()
        };
        Ok(pid)
    }

    pub fn new_named_pipe(path: &str) -> Result<Session> {
        debug!("Creating named piped session: {}", path);
        let session = unsafe {
            let mut handle = MaybeUninit::uninit();
            let cs = CString::new(path)?;
            ffi::HAPI_CreateThriftNamedPipeSession(handle.as_mut_ptr(), cs.as_ptr())
                .result_with_message("Could not start piped session")?;
            handle.assume_init()
        };
        Ok(Session {
            handle: Arc::new(session),
            unsync: false,
            cleanup: false,
        })
    }

    pub fn initialize(&mut self, opts: SessionOptions) -> Result<()> {
        debug!("Initializing session");
        self.unsync = opts.unsync;
        self.cleanup = opts.cleanup;
        unsafe {
            ffi::HAPI_Initialize(
                self.ptr(),
                opts.cook_opt.ptr(),
                opts.unsync as i8,
                -1,
                opts.env_files.map(|p| p.as_ptr()).unwrap_or(null()),
                opts.otl_path.map(|p| p.as_ptr()).unwrap_or(null()),
                opts.dso_path.map(|p| p.as_ptr()).unwrap_or(null()),
                opts.img_dso_path.map(|p| p.as_ptr()).unwrap_or(null()),
                opts.aud_dso_path.map(|p| p.as_ptr()).unwrap_or(null()),
            )
                .result_with_session(|| self.clone())
        }
    }

    pub fn cleanup(&self) -> Result<()> {
        debug!("Cleaning session");
        unsafe { ffi::HAPI_Cleanup(self.ptr()).result_with_session(|| self.clone()) }
    }

    pub fn close_session(&self) -> Result<()> {
        debug!("Closing session");
        unsafe { ffi::HAPI_CloseSession(self.ptr()).result_with_session(|| self.clone()) }
    }

    pub fn is_initialized(&self) -> Result<bool> {
        unsafe {
            match ffi::HAPI_IsInitialized(self.ptr()) {
                ffi::HapiResult::Success => Ok(true),
                ffi::HapiResult::NotInitialized => Ok(false),
                e => hapi_err!(e, None, Some("HAPI_IsInitialized failed")),
            }
        }
    }

    pub fn create_node_blocking(
        &self,
        name: &str,
        label: Option<&str>,
        parent: Option<NodeHandle>,
    ) -> Result<HoudiniNode> {
        HoudiniNode::create_blocking(name, label, parent, self.clone(), false)
    }

    pub fn save_hip(&self, name: &str) -> Result<()> {
        debug!("Saving hip file: {}", name);
        unsafe {
            let name = CString::new(name)?;
            ffi::HAPI_SaveHIPFile(self.ptr(), name.as_ptr(), 0).result_with_session(|| self.clone())
        }
    }

    pub fn load_hip(&self, name: &str, cook: bool) -> Result<()> {
        debug!("Loading hip file: {}", name);
        unsafe {
            let name = CString::new(name)?;
            ffi::HAPI_LoadHIPFile(self.ptr(), name.as_ptr(), cook as i8)
                .result_with_session(|| self.clone())
        }
    }

    pub fn merge_hip(&self, name: &str, cook: bool) -> Result<i32> {
        debug!("Merging hip file: {}", name);
        unsafe {
            let name = CString::new(name)?;
            let mut id = MaybeUninit::uninit();
            ffi::HAPI_MergeHIPFile(self.ptr(), name.as_ptr(), cook as i8, id.as_mut_ptr())
                .result_with_session(|| self.clone())?;
            Ok(id.assume_init())
        }
    }

    pub fn load_asset_file(&self, file: impl AsRef<str>) -> Result<AssetLibrary> {
        AssetLibrary::from_file(self.clone(), file)
    }

    pub fn interrupt(&self) -> Result<()> {
        unsafe { ffi::HAPI_Interrupt(self.ptr()).result_with_session(|| self.clone()) }
    }

    pub fn get_status(&self, flag: StatusType) -> Result<State> {
        let status = unsafe {
            let mut status = MaybeUninit::uninit();
            ffi::HAPI_GetStatus(self.ptr(), flag.into(), status.as_mut_ptr())
                .result_with_session(|| self.clone())?;
            status.assume_init()
        };
        Ok(State::from(status))
    }

    pub fn is_cooking(&self) -> Result<bool> {
        Ok(matches!(
            self.get_status(StatusType::CookState)?,
            State::Cooking
        ))
    }

    pub fn is_valid(&self) -> Result<bool> {
        unsafe {
            let res = ffi::HAPI_IsSessionValid(self.ptr());
            hapi_result!(res, true, None, Some("Session::is_valid failed"))
        }
    }

    pub fn get_string(&self, handle: i32) -> Result<String> {
        crate::stringhandle::get_string(handle, self)
    }

    pub fn get_status_string(
        &self,
        status: StatusType,
        verbosity: StatusVerbosity,
    ) -> Result<String> {
        unsafe {
            let mut length = std::mem::MaybeUninit::uninit();
            ffi::HAPI_GetStatusStringBufLength(
                self.ptr(),
                status.into(),
                verbosity.into(),
                length.as_mut_ptr(),
            )
                .result_with_message("GetStatusStringBufLength failed")?;
            let length = length.assume_init();
            let mut buf = vec![0u8; length as usize - 1];
            if length > 0 {
                ffi::HAPI_GetStatusString(
                    self.ptr(),
                    status.into(),
                    buf.as_mut_ptr() as *mut i8,
                    length,
                )
                    .result_with_message("GetStatusString failed")?;
                buf.truncate(length as usize);
                Ok(String::from_utf8_unchecked(buf))
            } else {
                Ok(String::new())
            }
        }
    }

    pub fn get_cook_status(&self, verbosity: StatusVerbosity) -> Result<String> {
        self.get_status_string(StatusType::CookResult, verbosity)
    }

    pub fn cooking_total_count(&self) -> Result<i32> {
        unsafe {
            let mut count = MaybeUninit::uninit();
            ffi::HAPI_GetCookingTotalCount(self.ptr(), count.as_mut_ptr())
                .result_with_session(|| self.clone())?;
            Ok(count.assume_init())
        }
    }

    pub fn cooking_current_count(&self) -> Result<i32> {
        unsafe {
            let mut count = MaybeUninit::uninit();
            ffi::HAPI_GetCookingCurrentCount(self.ptr(), count.as_mut_ptr())
                .result_with_session(|| self.clone())?;
            Ok(count.assume_init())
        }
    }

    pub fn cook_result(&self) -> Result<CookResult> {
        if self.unsync {
            loop {
                match self.get_status(StatusType::CookState)? {
                    State::Ready => break Ok(CookResult::Succeeded),
                    State::ReadyWithFatalErrors => {
                        self.interrupt()?;
                        let err = self.get_cook_status(StatusVerbosity::Errors)?;
                        break Ok(CookResult::Errored(err));
                    }
                    State::ReadyWithCookErrors => break Ok(CookResult::Warnings),
                    _ => {}
                }
            }
        } else {
            Ok(CookResult::Succeeded)
        }
    }

    pub fn get_connection_error(&self, clear: bool) -> Result<String> {
        unsafe {
            let mut length = MaybeUninit::uninit();
            ffi::HAPI_GetConnectionErrorLength(length.as_mut_ptr())
                .result_with_message("HAPI_GetConnectionErrorLength failed")?;
            let length = length.assume_init();
            if length > 0 {
                let mut buf = vec![0u8; length as usize];
                ffi::HAPI_GetConnectionError(buf.as_mut_ptr() as *mut _, length, clear as i8)
                    .result_with_message("HAPI_GetConnectionError failed")?;
                Ok(String::from_utf8_unchecked(buf))
            } else {
                Ok(String::new())
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if Arc::strong_count(&self.handle) == 1 {
            check_session!(self.ptr());
            unsafe {
                use ffi::HapiResult::*;
                if self.cleanup {
                    if let Err(e) = self.cleanup() {
                        error!("Cleanup failed in Drop: {}", e);
                    }
                }
                if let Err(e) = self.close_session() {
                    error!("Closing session failed in Drop: {}", e);
                }
            }
        }
    }
}

fn join_paths<I>(files: I) -> String
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
{
    let mut buf = String::new();
    let mut iter = files.into_iter().peekable();
    while let Some(n) = iter.next() {
        buf.push_str(n.as_ref());
        if iter.peek().is_some() {
            buf.push(':');
        }
    }
    buf
}

pub struct SessionOptions {
    pub cook_opt: CookOptions,
    pub unsync: bool,
    pub cleanup: bool,
    pub env_files: Option<CString>,
    pub otl_path: Option<CString>,
    pub dso_path: Option<CString>,
    pub img_dso_path: Option<CString>,
    pub aud_dso_path: Option<CString>,
}

impl Default for SessionOptions {
    fn default() -> Self {
        SessionOptions {
            cook_opt: CookOptions::default(),
            unsync: true,
            cleanup: false,
            env_files: None,
            otl_path: None,
            dso_path: None,
            img_dso_path: None,
            aud_dso_path: None,
        }
    }
}

impl SessionOptions {
    pub fn set_houdini_env_files<I>(&mut self, files: I)
        where
            I: IntoIterator,
            I::Item: AsRef<str>,
    {
        let paths = join_paths(files);
        self.env_files.replace(CString::new(paths).expect("Zero byte"));
    }

    pub fn set_otl_search_paths<I>(&mut self, paths: I)
        where
            I: IntoIterator,
            I::Item: AsRef<str>,
    {
        let paths = join_paths(paths);
        self.otl_path.replace(CString::new(paths).expect("Zero byte"));
    }

    pub fn set_dso_search_paths<P>(&mut self, paths: P)
        where
            P: IntoIterator,
            P::Item: AsRef<str>,
    {
        let paths = join_paths(paths);
        self.dso_path.replace(CString::new(paths).expect("Zero byte"));
    }

    pub fn set_image_search_paths<P>(&mut self, paths: P)
        where
            P: IntoIterator,
            P::Item: AsRef<str>,
    {
        let paths = join_paths(paths);
        self.img_dso_path
            .replace(CString::new(paths).expect("Zero byte"));
    }

    pub fn set_audio_search_paths<P>(&mut self, paths: P)
        where
            P: IntoIterator,
            P::Item: AsRef<str>,
    {
        let paths = join_paths(paths);
        self.aud_dso_path.replace(CString::new(paths).expect("Zero byte"));
    }
}
