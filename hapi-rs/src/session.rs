use crate::{
    asset::AssetLibrary,
    auto::rusty::{State, StatusType, StatusVerbosity},
    check_session,
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
    path::Path
};

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
        let mut ses = MaybeUninit::uninit();
        unsafe {
            match ffi::HAPI_CreateInProcessSession(ses.as_mut_ptr()) {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(Session {
                    handle: Arc::new(ses.assume_init()),
                    unsync: false,
                    cleanup: true,
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
                .result_with_message(Some("Could not start thrift server"))?;
            pid.assume_init()
        };
        Ok(pid)
    }

    pub fn new_named_pipe(path: &str) -> Result<Session> {
        let session = unsafe {
            let mut handle = MaybeUninit::uninit();
            let cs = CString::new(path)?;
            ffi::HAPI_CreateThriftNamedPipeSession(handle.as_mut_ptr(), cs.as_ptr())
                .result_with_message(Some("Could not start piped session"))?;
            handle.assume_init()
        };
        Ok(Session {
            handle: Arc::new(session),
            unsync: false,
            cleanup: false,
        })
    }

    pub fn initialize(&mut self, opts: SessionOptions) -> Result<()> {
        unsafe {
            ffi::HAPI_Initialize(
                self.ptr(),
                opts.cook_opt.const_ptr(),
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

    pub fn is_initialized(&self) -> Result<bool> {
        unsafe {
            match ffi::HAPI_IsInitialized(self.ptr()) {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(true),
                ffi::HAPI_Result::HAPI_RESULT_NOT_INITIALIZED => Ok(false),
                e => hapi_err!(e, None, Some("HAPI_IsInitialized failed")),
            }
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
            ffi::HAPI_SaveHIPFile(self.ptr(), name.as_ptr(), 0).result_with_session(|| self.clone())
        }
    }

    pub fn load_hip(&self, name: &str, cook: bool) -> Result<()> {
        unsafe {
            let name = CString::new(name)?;
            ffi::HAPI_LoadHIPFile(self.ptr(), name.as_ptr(), cook as i8)
                .result_with_session(|| self.clone())
        }
    }

    pub fn merge_hip(&self, name: &str, cook: bool) -> Result<i32> {
        unsafe {
            let name = CString::new(name)?;
            let mut id = MaybeUninit::uninit();
            ffi::HAPI_MergeHIPFile(self.ptr(), name.as_ptr(), cook as i8, id.as_mut_ptr())
                .result_with_session(|| self.clone())?;
            Ok(id.assume_init())
        }
    }

    pub fn load_asset_file(&self, file: &str) -> Result<AssetLibrary> {
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

    pub fn last_cook_error(&self, verbosity: Option<StatusVerbosity>) -> Result<String> {
        let verbosity = verbosity.unwrap_or(StatusVerbosity::VerbosityErrors);
        get_cook_status(self, verbosity)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if Arc::strong_count(&self.handle) == 1 {
            eprintln!("Dropping last Session");
            check_session!(self.ptr());
            unsafe {
                use ffi::HAPI_Result::*;
                if self.cleanup {
                    eprintln!("HAPI_Cleanup");
                    if !matches!(ffi::HAPI_Cleanup(self.ptr()), HAPI_RESULT_SUCCESS) {
                        eprintln!("HAPI_Cleanup failed!");
                    }
                }
                if !matches!(ffi::HAPI_CloseSession(self.ptr()), HAPI_RESULT_SUCCESS) {
                    eprintln!("HAPI_CloseSession failed!");
                }
            }
        }
    }
}

fn join_paths<I>(files: I) -> String
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    let mut buf = String::new();
    let mut iter = files.into_iter().peekable();
    while let Some(n) = iter.next() {
        buf.push_str(&n.as_ref().to_string_lossy());
        if iter.peek().is_some() {
            buf.push(':');
        }
    }
    buf
}

pub struct SessionOptions {
    cook_opt: CookOptions,
    unsync: bool,
    cleanup: bool,
    env_files: Option<CString>,
    otl_path: Option<CString>,
    dso_path: Option<CString>,
    img_dso_path: Option<CString>,
    aud_dso_path: Option<CString>,
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
    // pub fn set_houdini_env_files<Files>(&mut self, files: Files)
    //     where
    //         Files: IntoIterator,
    //         Files::Item: AsRef<Path>,
    // {
    //     let paths = join_paths(files);
    //     self.env_files
    //         .replace(CString::new(paths).expect("Zero byte"));
    // }

    pub fn otl_search_paths<I>(mut self, paths: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        let paths = join_paths(paths);
        self.otl_path
            .replace(CString::new(paths).expect("set_otl_search_paths: zero byte in string"));
        self
    }

    // pub fn set_dso_search_paths<P>(&mut self, paths: P)
    //     where
    //         P: IntoIterator,
    //         P::Item: AsRef<Path>,
    // {
    //     let paths = join_paths(paths);
    //     self.dso_path
    //         .replace(CString::new(paths).expect("Zero byte"));
    // }
    //
    // pub fn set_image_search_paths<P>(&mut self, paths: P)
    //     where
    //         P: IntoIterator,
    //         P::Item: AsRef<Path>,
    // {
    //     let paths = join_paths(paths);
    //     self.img_dso_path
    //         .replace(CString::new(paths).expect("Zero byte"));
    // }
    //
    // pub fn set_audio_search_paths<P>(&mut self, paths: P)
    //     where
    //         P: IntoIterator,
    //         P::Item: AsRef<Path>,
    // {
    //     let paths = join_paths(paths);
    //     self.aud_dso_path
    //         .replace(CString::new(paths).expect("Zero byte"));
    // }
    //
    // pub fn set_cook_thread(&mut self, thread: bool) {
    //     self.cook_thread = thread;
    // }
    // pub fn set_cook_options(&mut self, opts: &'a CookOptions) {
    //     self.cook_opt.replace(opts);
    // }
}
