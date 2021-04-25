#[rustfmt::skip]
use std::{
    ffi::CString,
    mem::MaybeUninit,
    ops::Deref,
    path::Path,
    ptr::null,
    sync::Arc,
};
use std::borrow::Cow;

use log::{debug, error, warn};

pub use crate::{
    asset::AssetLibrary,
    errors::*,
    ffi::raw::{HapiResult, State, StatusType, StatusVerbosity},
    ffi::{CookOptions, TimelineOptions, TimelineOptionsBuilder},
    node::{HoudiniNode, NodeHandle},
    stringhandle::StringsArray,
};

#[derive(Debug, Clone)]
pub enum CookResult {
    Succeeded,
    Warnings,
    Errored(String),
}

#[derive(Debug, Clone)]
pub struct Session {
    handle: Arc<crate::ffi::raw::HAPI_Session>,
    pub unsync: bool,
    cleanup: bool,
}

impl Session {
    #[inline]
    pub(crate) fn ptr(&self) -> *const crate::ffi::raw::HAPI_Session {
        self.handle.as_ref() as *const _
    }
    #[inline]
    pub fn uid(&self) -> i64 {
        self.handle.id
    }

    pub fn new_in_process() -> Result<Session> {
        debug!("Creating new in-process session");
        let ses = crate::ffi::create_inprocess_session()?;
        Ok(Session {
            handle: Arc::new(ses),
            unsync: false,
            cleanup: true,
        })
    }

    pub fn set_server_env(&self, key: &str, value: &str) -> Result<()> {
        let key = CString::new(key)?;
        let value = CString::new(value)?;
        crate::ffi::set_server_env_variable(self, &key, &value)
    }

    pub fn get_server_env(&self, key: &str) -> Result<String> {
        crate::ffi::get_server_env_variable(self, &CString::new(key)?)
    }

    pub fn get_server_env_variables(&self) -> Result<StringsArray> {
        let count = crate::ffi::get_server_env_var_count(self)?;
        let handles = crate::ffi::get_server_env_var_list(self, count)?;
        crate::stringhandle::get_strings_array(&handles, self)
    }

    pub fn connect_to_pipe(pipe: &str) -> Result<Session> {
        debug!("Connecting to Thrift session: {}", pipe);
        let path = CString::new(pipe)?;
        let session = crate::ffi::new_thrift_piped_session(&path)?;
        Ok(Session {
            handle: Arc::new(session),
            unsync: false,
            cleanup: false,
        })
    }

    pub fn connect_to_socket(addr: std::net::SocketAddrV4) -> Result<Session> {
        debug!("Connecting to socket server: {:?}", addr);
        let host = CString::new(addr.ip().to_string()).expect("SocketAddr->CString");
        let session = crate::ffi::new_thrift_socket_session(addr.port() as i32, &host)?;
        Ok(Session {
            handle: Arc::new(session),
            unsync: false,
            cleanup: false,
        })
    }

    pub fn initialize(&mut self, opts: &SessionOptions) -> Result<()> {
        debug!("Initializing session");
        self.unsync = opts.unsync;
        self.cleanup = opts.cleanup;
        crate::ffi::initialize_session(self.handle.as_ref(), opts)
    }

    pub fn cleanup(&self) -> Result<()> {
        debug!("Cleaning session");
        crate::ffi::cleanup_session(self)
    }

    pub fn close_session(&self) -> Result<()> {
        debug!("Closing session");
        crate::ffi::close_session(self)
    }

    pub fn is_initialized(&self) -> Result<bool> {
        crate::ffi::is_session_initialized(self)
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
        let name = CString::new(name)?;
        crate::ffi::save_hip(self, &name)
    }

    pub fn load_hip(&self, name: &str, cook: bool) -> Result<()> {
        debug!("Loading hip file: {}", name);
        let name = CString::new(name)?;
        crate::ffi::load_hip(self, &name, cook)
    }

    pub fn merge_hip(&self, name: &str, cook: bool) -> Result<i32> {
        debug!("Merging hip file: {}", name);
        let name = CString::new(name)?;
        crate::ffi::merge_hip(self, &name, cook)
    }

    pub fn load_asset_file(&self, file: impl AsRef<str>) -> Result<AssetLibrary> {
        AssetLibrary::from_file(self.clone(), file)
    }

    pub fn interrupt(&self) -> Result<()> {
        crate::ffi::interrupt(self)
    }

    pub fn get_status(&self, flag: StatusType) -> Result<State> {
        crate::ffi::get_status(self, flag)
    }

    pub fn is_cooking(&self) -> Result<bool> {
        Ok(matches!(
            self.get_status(StatusType::CookState)?,
            State::Cooking
        ))
    }

    pub fn is_valid(&self) -> bool {
        crate::ffi::is_session_valid(self)
    }

    pub fn get_string(&self, handle: i32) -> Result<String> {
        crate::stringhandle::get_string(handle, self)
    }

    pub fn get_status_string(
        &self,
        status: StatusType,
        verbosity: StatusVerbosity,
    ) -> Result<String> {
        crate::ffi::get_status_string(self, status, verbosity)
    }

    pub fn get_cook_status(&self, verbosity: StatusVerbosity) -> Result<String> {
        self.get_status_string(StatusType::CookResult, verbosity)
    }

    pub fn cooking_total_count(&self) -> Result<i32> {
        crate::ffi::get_cooking_total_count(self)
    }

    pub fn cooking_current_count(&self) -> Result<i32> {
        crate::ffi::get_cooking_current_count(self)
    }

    pub fn cook(&self) -> Result<CookResult> {
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
        crate::ffi::get_connection_error(self, clear)
    }

    pub fn get_time(&self) -> Result<f32> {
        crate::ffi::get_time(self)
    }

    pub fn set_time(&self, time: f32) -> Result<()> {
        crate::ffi::set_time(self, time)
    }

    pub fn set_timeline_options(&self, options: TimelineOptions) -> Result<()> {
        crate::ffi::set_timeline_options(self, &options.inner)
    }

    pub fn get_timeline_options(&self) -> Result<TimelineOptions> {
        crate::ffi::get_timeline_options(self).map(|opt| TimelineOptions { inner: opt })
    }

    pub fn set_use_houdini_time(&self, do_use: bool) -> Result<()> {
        crate::ffi::set_use_houdini_time(self, do_use)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if Arc::strong_count(&self.handle) == 1 {
            eprintln!("Dropping session");
            assert!(self.is_valid(), "Session invalid in Drop");
            unsafe {
                use crate::ffi::raw::HapiResult::*;
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

/// Join a sequence of paths into a single String
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
        self.env_files
            .replace(CString::new(paths).expect("Zero byte"));
    }

    pub fn set_otl_search_paths<I>(&mut self, paths: I)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let paths = join_paths(paths);
        self.otl_path
            .replace(CString::new(paths).expect("Zero byte"));
    }

    pub fn set_dso_search_paths<P>(&mut self, paths: P)
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        let paths = join_paths(paths);
        self.dso_path
            .replace(CString::new(paths).expect("Zero byte"));
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
        self.aud_dso_path
            .replace(CString::new(paths).expect("Zero byte"));
    }
}

impl From<i32> for State {
    fn from(s: i32) -> Self {
        match s {
            0 => State::Ready,
            1 => State::ReadyWithFatalErrors,
            2 => State::ReadyWithCookErrors,
            3 => State::StartingCook,
            4 => State::Cooking,
            5 => State::StartingLoad,
            6 => State::Loading,
            7 => State::Max,
            _ => unreachable!(),
        }
    }
}

pub fn start_engine_pipe_server(path: &str, auto_close: bool, timeout: f32) -> Result<i32> {
    debug!("Starting named pipe server: {}", path);
    let path = CString::new(path)?;
    let opts = crate::ffi::raw::HAPI_ThriftServerOptions {
        autoClose: auto_close as i8,
        timeoutMs: timeout,
    };
    crate::ffi::start_thrift_pipe_server(&path, &opts)
}
pub fn start_engine_socket_server(port: u16, auto_close: bool, timeout: i32) -> Result<i32> {
    debug!("Starting socket server on port: {}", port);
    let opts = crate::ffi::raw::HAPI_ThriftServerOptions {
        autoClose: auto_close as i8,
        timeoutMs: timeout as f32,
    };
    crate::ffi::start_thrift_socket_server(port as i32, &opts)
}
