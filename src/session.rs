//! Session is responsible for communicating with HAPI
//!
//! The Engine [promises](https://www.sidefx.com/docs/hengine/_h_a_p_i__sessions.html#HAPI_Sessions_Multithreading)
//! to be thread-safe when accessing a single `Session` from multiple threads.
//! `hapi-rs` relies on this promise and the [Session] struct holds only an `Arc` pointer to the session,
//! and *does not* protect the session with Mutex, although there is a [parking_lot::ReentrantMutex]
//! private member which is used internally in a few cases where API calls must be sequential.
//!
//! When the last instance of the `Session` is about to drop, it'll be cleaned
//! (if [SessionOptions::cleanup] was set) and automatically closed.
//!
//! The Engine process (pipe or socket) can be auto-terminated as well if told so when starting the server:
//! See [start_engine_pipe_server] and [start_engine_socket_server]
//!
//! [quick_session] terminates the server by default. This is useful for quick one-off jobs.
//!
#[rustfmt::skip]
use std::{
    ffi::CString,
    sync::Arc,
    path::Path,
};
pub use crate::ffi::enums::*;
use log::{debug, error, warn};

use crate::{
    asset::AssetLibrary,
    errors::*,
    ffi::raw,
    ffi::{CookOptions, SessionSyncInfo, TimelineOptions, Viewport},
    node::{HoudiniNode, NodeHandle},
    stringhandle::StringArray,
};

use parking_lot::ReentrantMutex;

impl std::cmp::PartialEq for raw::HAPI_Session {
    fn eq(&self, other: &Self) -> bool {
        self.type_ == other.type_ && self.id == other.id
    }
}

/// Trait bound for [`Session::get_server_var()`] and [`Session::set_server_var()`]
pub trait EnvVariable {
    type Type: ?Sized + ToOwned;
    fn get_value(session: &Session, key: impl AsRef<str>)
        -> Result<<Self::Type as ToOwned>::Owned>;
    fn set_value(session: &Session, key: impl AsRef<str>, val: &Self::Type) -> Result<()>;
}

impl EnvVariable for str {
    type Type = str;

    fn get_value(session: &Session, key: impl AsRef<str>) -> Result<String> {
        let key = CString::new(key.as_ref())?;
        crate::stringhandle::get_string(crate::ffi::get_server_env_str(session, &key)?, session)
    }

    fn set_value(session: &Session, key: impl AsRef<str>, val: &Self::Type) -> Result<()> {
        let key = CString::new(key.as_ref())?;
        let val = CString::new(val)?;
        crate::ffi::set_server_env_str(session, &key, &val)
    }
}

impl EnvVariable for i32 {
    type Type = Self;

    fn get_value(session: &Session, key: impl AsRef<str>) -> Result<Self::Type> {
        let key = CString::new(key.as_ref())?;
        crate::ffi::get_server_env_int(session, &key)
    }

    fn set_value(session: &Session, key: impl AsRef<str>, val: &Self::Type) -> Result<()> {
        let key = CString::new(key.as_ref())?;
        crate::ffi::set_server_env_int(session, &key, *val)
    }
}

/// Result of async cook operation [`Session::cook`]
#[derive(Debug, Clone)]
pub enum CookResult {
    Succeeded,
    /// Some nodes cooked with warnings
    Warnings,
    /// One or more nodes could not cook properly
    Errored(String),
}

/// Session represents a unique connection to the Engine instance and all API calls require a valid session.
/// It implements [`Clone`] and is [`Send`] and [`Sync`]
#[derive(Debug, Clone)]
pub struct Session {
    pub(crate) handle: Arc<(raw::HAPI_Session, ReentrantMutex<()>)>,
    pub(crate) threaded: bool,
    cleanup: bool,
}

impl Session {
    #[inline]
    pub(crate) fn ptr(&self) -> *const raw::HAPI_Session {
        &(self.handle.0) as *const _
    }

    /// Set environment variable on the server
    pub fn set_server_var<T: EnvVariable + ?Sized>(
        &self,
        key: &str,
        value: &T::Type,
    ) -> Result<()> {
        debug_assert!(self.is_valid());
        T::set_value(self, key, value)
    }

    /// Get environment variable from the server
    pub fn get_server_var<T: EnvVariable + ?Sized>(
        &self,
        key: &str,
    ) -> Result<<T::Type as ToOwned>::Owned> {
        debug_assert!(self.is_valid());
        T::get_value(self, key)
    }

    /// Retrieve all server variables
    pub fn get_server_variables(&self) -> Result<StringArray> {
        debug_assert!(self.is_valid());
        let count = crate::ffi::get_server_env_var_count(self)?;
        let handles = crate::ffi::get_server_env_var_list(self, count)?;
        crate::stringhandle::get_string_array(&handles, self)
    }

    /// Initialize session with options
    pub fn initialize(&mut self, opts: &SessionOptions) -> Result<()> {
        debug!("Initializing session");
        debug_assert!(self.is_valid());
        self.threaded = opts.threaded;
        self.cleanup = opts.cleanup;
        let res = crate::ffi::initialize_session(self, opts);
        if !opts.ignore_already_init {
            return res;
        }
        match res {
            Ok(_) => Ok(()),
            Err(e) => match e {
                HapiError {
                    kind: Kind::Hapi(HapiResult::AlreadyInitialized),
                    ..
                } => {
                    warn!("Session already initialized, skipping");
                    Ok(())
                }
                e => Err(e),
            },
        }
    }

    /// Cleanup the session. Session will not be valid after this call
    /// and needs to be initialized again
    pub fn cleanup(&self) -> Result<()> {
        debug!("Cleaning session");
        debug_assert!(self.is_valid());
        crate::ffi::cleanup_session(self)
    }

    /// Check if session is initialized
    pub fn is_initialized(&self) -> bool {
        debug_assert!(self.is_valid());
        crate::ffi::is_session_initialized(self)
    }

    // TODO: Return a Geometry instead for convenient use
    /// Create an input geometry node which can accept modifications
    pub fn create_input_node(&self, name: &str) -> Result<crate::geometry::Geometry> {
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        let id = crate::ffi::create_input_node(self, &name)?;
        let node = HoudiniNode::new(self.clone(), NodeHandle(id, ()), None)?;
        let info = crate::geometry::GeoInfo::from_node(&node)?;
        Ok(crate::geometry::Geometry { node, info })
    }

    /// Create an input geometry node with [`crate::enums::PartType`] set to `Curve`
    pub fn create_input_curve_node(&self, name: &str) -> Result<crate::geometry::Geometry> {
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        let id = crate::ffi::create_input_curve_node(self, &name)?;
        let node = HoudiniNode::new(self.clone(), NodeHandle(id, ()), None)?;
        let info = crate::geometry::GeoInfo::from_node(&node)?;
        Ok(crate::geometry::Geometry { node, info })
    }

    /// Create a node. `name` must start with a network category, e.g, "Object/geo", "Sop/box"
    /// New node will not be cooked.
    pub fn create_node<'a>(
        &self,
        name: impl AsRef<str>,
        label: impl Into<Option<&'a str>>,
        parent: Option<NodeHandle>,
    ) -> Result<HoudiniNode> {
        debug_assert!(self.is_valid());
        HoudiniNode::create(name.as_ref(), label.into(), parent, self.clone(), false)
    }

    /// Find a node given an absolute path. To find a child node, pass the `parent` node
    /// or see [`HoudiniNode::get_child`]
    pub fn find_node(
        &self,
        path: impl AsRef<str>,
        parent: Option<NodeHandle>,
    ) -> Result<HoudiniNode> {
        let path = CString::new(path.as_ref())?;
        crate::ffi::get_node_from_path(self, parent, &path)
            .map(|id| NodeHandle(id, ()).to_node(self))?
    }

    /// Save current session to hip file
    pub fn save_hip(&self, name: &str, lock_nodes: bool) -> Result<()> {
        debug!("Saving hip file: {}", name);
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        crate::ffi::save_hip(self, &name, lock_nodes)
    }

    /// Load a hip file into current session
    pub fn load_hip(&self, name: &str, cook: bool) -> Result<()> {
        debug!("Loading hip file: {}", name);
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        crate::ffi::load_hip(self, &name, cook)
    }

    /// Merge a hip file into current session
    pub fn merge_hip(&self, name: &str, cook: bool) -> Result<i32> {
        debug!("Merging hip file: {}", name);
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        crate::ffi::merge_hip(self, &name, cook)
    }

    /// Load an HDA file into current session
    pub fn load_asset_file(&self, file: impl AsRef<str>) -> Result<AssetLibrary> {
        debug_assert!(self.is_valid());
        AssetLibrary::from_file(self.clone(), file)
    }

    /// Interrupt session cooking
    pub fn interrupt(&self) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::interrupt(self)
    }

    /// Get session state of a requested [`create::enums::StatusType`]
    pub fn get_status(&self, flag: StatusType) -> Result<State> {
        debug_assert!(self.is_valid());
        crate::ffi::get_status(self, flag)
    }

    /// Is session currently cooking. In non-threaded mode always returns false
    pub fn is_cooking(&self) -> Result<bool> {
        debug_assert!(self.is_valid());
        Ok(matches!(
            self.get_status(StatusType::CookState)?,
            State::Cooking
        ))
    }

    pub fn is_valid(&self) -> bool {
        crate::ffi::is_session_valid(self)
    }

    /// Get the status message given a type and verbosity
    pub fn get_status_string(
        &self,
        status: StatusType,
        verbosity: StatusVerbosity,
    ) -> Result<String> {
        debug_assert!(self.is_valid());
        crate::ffi::get_status_string(self, status, verbosity)
    }

    /// Get session cook result status as string
    pub fn get_cook_result_string(&self, verbosity: StatusVerbosity) -> Result<String> {
        debug_assert!(self.is_valid());
        self.get_status_string(StatusType::CookResult, verbosity)
    }

    /// How many nodes need to cook
    pub fn cooking_total_count(&self) -> Result<i32> {
        debug_assert!(self.is_valid());
        crate::ffi::get_cooking_total_count(self)
    }

    /// How many nodes have already cooked
    pub fn cooking_current_count(&self) -> Result<i32> {
        debug_assert!(self.is_valid());
        crate::ffi::get_cooking_current_count(self)
    }

    /// In threaded mode wait for Session finishes cooking. In single-thread mode, immediately return
    /// See [Documentation](https://www.sidefx.com/docs/hengine/_h_a_p_i__sessions.html)
    pub fn cook(&self) -> Result<CookResult> {
        debug_assert!(self.is_valid());
        if self.threaded {
            loop {
                match self.get_status(StatusType::CookState)? {
                    State::Ready => break Ok(CookResult::Succeeded),
                    State::ReadyWithFatalErrors => {
                        self.interrupt()?;
                        let err = self.get_cook_result_string(StatusVerbosity::Errors)?;
                        break Ok(CookResult::Errored(err));
                    }
                    State::ReadyWithCookErrors => break Ok(CookResult::Warnings),
                    // Continue polling
                    _ => {}
                }
            }
        } else {
            // In single threaded mode, the cook happens inside of HAPI_CookNode(),
            // and HAPI_GetStatus() will immediately return HAPI_STATE_READY.
            Ok(CookResult::Succeeded)
        }
    }

    /// Retrieve connection error if could not connect to engine instance
    pub fn get_connection_error(&self, clear: bool) -> Result<String> {
        debug_assert!(self.is_valid());
        crate::ffi::get_connection_error(clear)
    }

    /// Get Houdini time
    pub fn get_time(&self) -> Result<f32> {
        debug_assert!(self.is_valid());
        crate::ffi::get_time(self)
    }

    /// Set Houdini time
    pub fn set_time(&self, time: f32) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::set_time(self, time)
    }

    /// Lock the internal reentrant mutex. Should not be used in general, but may be useful
    /// in certain situations when a series of API calls must be done in sequence
    pub fn lock(&self) -> parking_lot::ReentrantMutexGuard<()> {
        self.handle.1.lock()
    }

    /// Set Houdini timeline options
    pub fn set_timeline_options(&self, options: TimelineOptions) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::set_timeline_options(self, &options.inner)
    }

    /// Get Houdini timeline options
    pub fn get_timeline_options(&self) -> Result<TimelineOptions> {
        debug_assert!(self.is_valid());
        crate::ffi::get_timeline_options(self).map(|opt| TimelineOptions { inner: opt })
    }

    /// Set session to use Houdini time
    pub fn set_use_houdini_time(&self, do_use: bool) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::set_use_houdini_time(self, do_use)
    }

    /// Get the viewport(camera) position
    pub fn get_viewport(&self) -> Result<Viewport> {
        debug_assert!(self.is_valid());
        crate::ffi::get_viewport(self).map(|inner| Viewport { inner })
    }

    /// Set the viewport(camera) position
    pub fn set_viewport(&self, viewport: &Viewport) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::set_viewport(self, viewport)
    }

    /// Set session sync mode on/off
    pub fn set_sync(&self, enable: bool) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::set_session_sync(self, enable)
    }
    /// Get session sync info
    pub fn get_sync_info(&self) -> Result<SessionSyncInfo> {
        debug_assert!(self.is_valid());
        crate::ffi::get_session_sync_info(self).map(|inner| SessionSyncInfo { inner })
    }

    /// Set session sync info
    pub fn set_sync_info(&self, info: &SessionSyncInfo) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::set_session_sync_info(self, &info.inner)
    }

    /// Get license type used by this session
    pub fn get_license_type(&self) -> Result<License> {
        debug_assert!(self.is_valid());
        crate::ffi::session_get_license_type(self)
    }

    /// Render a COP node to an image file
    pub fn render_cop_to_image(
        &self,
        cop_node: impl Into<NodeHandle>,
        image_planes: impl AsRef<str>,
        path: impl AsRef<Path>,
    ) -> Result<String> {
        let cop_node = cop_node.into();
        debug_assert!(cop_node.is_valid(self)?);
        crate::ffi::render_cop_to_image(self, cop_node)?;
        crate::material::extract_image_to_file(self, cop_node, image_planes, path)
    }

    /// Render a COP node to a memory buffer
    pub fn render_cop_to_memory(
        &self,
        cop_node: impl Into<NodeHandle>,
        image_planes: impl AsRef<str>,
        format: impl AsRef<str>,
    ) -> Result<Vec<i8>> {
        let cop_node = cop_node.into();
        debug_assert!(cop_node.is_valid(self)?);
        crate::ffi::render_cop_to_image(self, cop_node)?;
        crate::material::extract_image_to_memory(self, cop_node, image_planes, format)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if Arc::strong_count(&self.handle) == 1 {
            debug!("Dropping session");
            if self.is_valid() {
                if self.cleanup {
                    debug!("Cleaning up session");
                    if let Err(e) = self.cleanup() {
                        error!("Cleanup failed in Drop: {}", e);
                    }
                }
                if let Err(e) = crate::ffi::shutdown_session(self) {
                    error!("Could not shutdown session in Drop: {}", e);
                }
                if let Err(e) = crate::ffi::close_session(self) {
                    error!("Closing session failed in Drop: {}", e);
                }
            }
        }
    }
}

/// Connect to the engine process via a Unix pipe
pub fn connect_to_pipe(pipe: &str) -> Result<Session> {
    debug!("Connecting to Thrift session: {}", pipe);
    let path = CString::new(pipe)?;
    let session = crate::ffi::new_thrift_piped_session(&path)?;
    Ok(Session {
        handle: Arc::new((session, ReentrantMutex::new(()))),
        threaded: false,
        cleanup: false,
    })
}

/// Connect to the engine process via a Unix socket
pub fn connect_to_socket(addr: std::net::SocketAddrV4) -> Result<Session> {
    debug!("Connecting to socket server: {:?}", addr);
    let host = CString::new(addr.ip().to_string()).expect("SocketAddr->CString");
    let session = crate::ffi::new_thrift_socket_session(addr.port() as i32, &host)?;
    Ok(Session {
        handle: Arc::new((session, ReentrantMutex::new(()))),
        threaded: false,
        cleanup: false,
    })
}

/// Create in-process session
pub fn new_in_process() -> Result<Session> {
    debug!("Creating new in-process session");
    let session = crate::ffi::create_inprocess_session()?;
    Ok(Session {
        handle: Arc::new((session, ReentrantMutex::new(()))),
        threaded: false,
        cleanup: false,
    })
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

/// Session options used in [`Session::initialize`]
// TODO: Add builder
pub struct SessionOptions {
    /// Session cook options
    pub cook_opt: CookOptions,
    /// Create a Threaded server connection
    pub threaded: bool,
    /// Cleanup session upon close
    pub cleanup: bool,
    /// Do not error out if session is already initialized
    pub ignore_already_init: bool,
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
            threaded: false,
            cleanup: false,
            ignore_already_init: true,
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

/// Spawn a new pipe Engine process and return its PID
pub fn start_engine_pipe_server(
    path: &str,
    auto_close: bool,
    timeout: f32,
    verbosity: StatusVerbosity,
    log_file: Option<&str>,
) -> Result<u32> {
    debug!("Starting named pipe server: {}", path);
    let path = CString::new(path)?;
    let opts = crate::ffi::raw::HAPI_ThriftServerOptions {
        autoClose: auto_close as i8,
        timeoutMs: timeout,
        verbosity,
    };
    let log_file = log_file.map(|p| CString::new(p)).transpose()?;
    crate::ffi::start_thrift_pipe_server(&path, &opts, log_file.as_deref())
}

/// Spawn a new socket Engine server and return its PID
pub fn start_engine_socket_server(
    port: u16,
    auto_close: bool,
    timeout: i32,
    verbosity: StatusVerbosity,
    log_file: Option<&str>,
) -> Result<u32> {
    debug!("Starting socket server on port: {}", port);
    let opts = crate::ffi::raw::HAPI_ThriftServerOptions {
        autoClose: auto_close as i8,
        timeoutMs: timeout as f32,
        verbosity,
    };
    let log_file = log_file.map(|p| CString::new(p)).transpose()?;
    crate::ffi::start_thrift_socket_server(port as i32, &opts, log_file.as_deref())
}

/// A quick drop-in session, useful for on-off jobs
/// It starts a single-threaded pipe server and initialize a session with default options
pub fn quick_session() -> Result<Session> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;
    let mut hash = DefaultHasher::new();
    SystemTime::now().hash(&mut hash);
    std::thread::current().id().hash(&mut hash);
    let file = std::env::temp_dir().join(format!("hars-session-{}", hash.finish()));
    let file = file.to_string_lossy();
    start_engine_pipe_server(&file, true, 4000.0, StatusVerbosity::Statusverbosity1, None)?;
    let mut session = connect_to_pipe(&file)?;
    session.initialize(&SessionOptions::default())?;
    Ok(session)
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::session::*;
    use once_cell::sync::Lazy;

    static SESSION: Lazy<Session> = Lazy::new(|| {
        env_logger::init();
        let session = quick_session().expect("Could not create test session");
        session
            .load_asset_file("otls/hapi_geo.hda")
            .expect("load asset");
        session
            .load_asset_file("otls/hapi_vol.hda")
            .expect("load asset");
        session
            .load_asset_file("otls/hapi_parms.hda")
            .expect("load asset");
        session
            .load_asset_file("otls/sesi/SideFX_spaceship.hda")
            .expect("load asset");
        session
    });

    pub(crate) fn with_session(func: impl FnOnce(&Lazy<Session>)) {
        func(&SESSION);
    }

    #[test]
    fn init_and_teardown() {
        let mut opt = super::SessionOptions::default();
        opt.set_dso_search_paths(["/path/one", "/path/two"]);
        opt.set_otl_search_paths(["/path/thee", "/path/four"]);
        let mut ses = super::quick_session().unwrap();
        assert!(ses.is_initialized());
        assert!(ses.is_valid());
        assert!(ses.cleanup().is_ok());
        assert!(!ses.is_initialized());
        ses.initialize(&opt).unwrap();
        assert!(ses.is_initialized());
    }

    #[test]
    fn session_time() {
        with_session(|session| {
            let _lock = session.lock();
            let opt = TimelineOptions::default().with_end_time(5.5);
            assert!(session.set_timeline_options(opt.clone()).is_ok());
            let opt2 = session.get_timeline_options().expect("timeline_options");
            assert!(opt.end_time().eq(&opt2.end_time()));
            session.set_time(4.12).expect("set_time");
            assert!(matches!(session.cook(), Ok(CookResult::Succeeded)));
            assert_eq!(session.get_time().expect("get_time"), 4.12);
        });
    }

    #[test]
    fn server_variables() {
        // Starting a new separate session because getting/setting env variables from multiple
        // clients ( threads ) breaks the server
        let session = super::quick_session().expect("Could not start session");
        session.set_server_var::<str>("FOO", "foo_string").unwrap();
        assert_eq!(session.get_server_var::<str>("FOO").unwrap(), "foo_string");
        session.set_server_var::<i32>("BAR", &123).unwrap();
        assert_eq!(session.get_server_var::<i32>("BAR").unwrap(), 123);
        assert!(!session.get_server_variables().unwrap().is_empty());
    }

    #[test]
    fn create_node_async() {
        use crate::ffi::raw::{NodeFlags, NodeType};
        let mut opt = super::SessionOptions::default();
        opt.threaded = true;
        let session = super::quick_session().unwrap();
        session
            .load_asset_file("otls/sesi/SideFX_spaceship.hda")
            .unwrap();
        let node = session.create_node("Object/spaceship", None, None).unwrap();
        assert_eq!(
            node.cook_count(NodeType::None, NodeFlags::None, true)
                .unwrap(),
            0
        );
        node.cook(None).unwrap(); // in threaded mode always successful
        assert_eq!(
            node.cook_count(NodeType::None, NodeFlags::None, true)
                .unwrap(),
            1
        );
        assert!(matches!(
            session.cook().unwrap(),
            super::CookResult::Succeeded
        ));
    }

    #[test]
    fn viewport() {
        with_session(|session| {
            let vp = Viewport::default()
                .with_rotation([0.7, 0.7, 0.7, 0.7])
                .with_position([0.0, 1.0, 0.0])
                .with_offset(3.5);
            session.set_viewport(&vp).expect("set_viewport");
            let vp2 = session.get_viewport().expect("get_viewport");
            assert_eq!(vp.position(), vp2.position());
            assert_eq!(vp.rotation(), vp2.rotation());
            assert_eq!(vp.offset(), vp2.offset());
        });
    }

    #[test]
    fn sync_session() {
        with_session(|session| {
            let info = SessionSyncInfo::default()
                .with_sync_viewport(true)
                .with_cook_using_houdini_time(true);
            session.set_sync_info(&info).unwrap();
            session.cook().unwrap();
            let info = session.get_sync_info().unwrap();
            assert!(info.sync_viewport());
            assert!(info.cook_using_houdini_time());
        });
    }
}
