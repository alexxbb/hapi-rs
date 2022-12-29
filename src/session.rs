//! Session is responsible for communicating with HAPI
//!
//! The Engine [promises](https://www.sidefx.com/docs/hengine/_h_a_p_i__sessions.html#HAPI_Sessions_Multithreading)
//! to be thread-safe when accessing a single `Session` from multiple threads.
//! `hapi-rs` relies on this promise and the [Session] struct holds only an `Arc` pointer to the session,
//! and *does not* protect the session with Mutex, although there is a [parking_lot::ReentrantMutex]
//! private member which is used internally in a few cases where API calls must be sequential.
//!
//! When the last instance of the `Session` is about to get dropped, it'll be cleaned up
//! (if [SessionOptions::cleanup] was set) and automatically closed.
//!
//! The Engine process (pipe or socket) can be auto-terminated as well if told so when starting the server:
//! See [start_engine_pipe_server] and [start_engine_socket_server]
//!
//! [quick_session] terminates the server by default. This is useful for quick one-off jobs.
//!
use log::{debug, error, warn};
use parking_lot::ReentrantMutex;
use std::ffi::OsString;
use std::fmt::Debug;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;
use std::{ffi::CString, path::Path, sync::Arc};

pub use crate::{
    asset::AssetLibrary,
    errors::*,
    ffi::{enums::*, CookOptions, ImageFileFormat, SessionSyncInfo, TimelineOptions, Viewport},
    node::{HoudiniNode, ManagerNode, ManagerType, NodeHandle, NodeType},
    parameter::Parameter,
    stringhandle::StringArray,
};
use crate::{ffi::raw, utils};

/// Builder struct for [`Session::node_builder`] API
pub struct NodeBuilder<'s> {
    session: &'s Session,
    name: String,
    label: Option<String>,
    parent: Option<NodeHandle>,
    cook: bool,
}

impl<'s> NodeBuilder<'s> {
    /// Give new node a label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Create new node as child of a parent node.
    pub fn with_parent<H: AsRef<NodeHandle>>(mut self, parent: H) -> Self {
        self.parent.replace(*parent.as_ref());
        self
    }

    /// Cook node after creation.
    pub fn cook(mut self, cook: bool) -> Self {
        self.cook = cook;
        self
    }

    /// Consume the builder and create the node
    pub fn create(self) -> Result<HoudiniNode> {
        let NodeBuilder {
            session,
            name,
            label,
            parent,
            cook,
        } = self;
        session.create_node_with(&name, parent, label.as_deref(), cook)
    }
}

impl PartialEq for raw::HAPI_Session {
    fn eq(&self, other: &Self) -> bool {
        self.type_ == other.type_ && self.id == other.id
    }
}

/// Trait bound for [`Session::get_server_var()`] and [`Session::set_server_var()`]
pub trait EnvVariable {
    type Type: ?Sized + ToOwned + Debug;
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

impl EnvVariable for Path {
    type Type = Self;

    fn get_value(session: &Session, key: impl AsRef<str>) -> Result<PathBuf> {
        let key = CString::new(key.as_ref())?;
        crate::stringhandle::get_string(crate::ffi::get_server_env_str(session, &key)?, session)
            .map(PathBuf::from)
    }

    fn set_value(session: &Session, key: impl AsRef<str>, val: &Self::Type) -> Result<()> {
        let key = CString::new(key.as_ref())?;
        let val = path_to_cstring(val)?;
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

/// By which means the session communicates with the server.
#[derive(Debug, Clone)]
pub enum ConnectionType {
    ThriftPipe(OsString),
    ThriftSocket(std::net::SocketAddrV4),
    InProcess,
    Custom,
}

#[derive(Debug)]
pub(crate) struct SessionInner {
    pub(crate) handle: raw::HAPI_Session,
    pub(crate) options: SessionOptions,
    pub(crate) connection: ConnectionType,
    pub(crate) lock: ReentrantMutex<()>,
}

/// Session represents a unique connection to the Engine instance and all API calls require a valid session.
/// It implements [`Clone`] and is [`Send`] and [`Sync`]
#[derive(Debug, Clone)]
pub struct Session {
    pub(crate) inner: Arc<SessionInner>,
}

impl PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.inner.handle.id == other.inner.handle.id
            && self.inner.handle.type_ == other.inner.handle.type_
    }
}

impl Session {
    fn new(
        handle: raw::HAPI_Session,
        connection: ConnectionType,
        options: SessionOptions,
    ) -> Session {
        Session {
            inner: Arc::new(SessionInner {
                handle,
                options,
                connection,
                lock: ReentrantMutex::new(()),
            }),
        }
    }

    /// Return [`SessionType`] current session is initialized with.
    pub fn session_type(&self) -> SessionType {
        self.inner.handle.type_
    }

    /// Return enum with extra connection data such as pipe file or socket.
    pub fn connection_type(&self) -> &ConnectionType {
        &self.inner.connection
    }

    #[inline]
    pub(crate) fn ptr(&self) -> *const raw::HAPI_Session {
        &(self.inner.handle) as *const _
    }

    /// Set environment variable on the server
    pub fn set_server_var<T: EnvVariable + ?Sized>(
        &self,
        key: &str,
        value: &T::Type,
    ) -> Result<()> {
        debug_assert!(self.is_valid());
        debug!("Setting server variable {key}={value:?}");
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

    /// Retrieve string data given a handle.
    pub fn get_string(&self, handle: i32) -> Result<String> {
        crate::stringhandle::get_string(handle, self)
    }

    fn initialize(&self) -> Result<()> {
        debug!("Initializing session");
        debug_assert!(self.is_valid());
        let res = crate::ffi::initialize_session(self, &self.inner.options);
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

    /// Create an input geometry node which can accept modifications
    pub fn create_input_node(&self, name: &str) -> Result<crate::geometry::Geometry> {
        debug!("Creating input node: {}", name);
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        let id = crate::ffi::create_input_node(self, &name)?;
        let node = HoudiniNode::new(self.clone(), NodeHandle(id), None)?;
        let info = crate::geometry::GeoInfo::from_node(&node)?;
        Ok(crate::geometry::Geometry { node, info })
    }

    /// Create an input geometry node with [`crate::enums::PartType`] set to `Curve`
    pub fn create_input_curve_node(&self, name: &str) -> Result<crate::geometry::Geometry> {
        debug!("Creating input curve node: {}", name);
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        let id = crate::ffi::create_input_curve_node(self, &name)?;
        let node = HoudiniNode::new(self.clone(), NodeHandle(id), None)?;
        let info = crate::geometry::GeoInfo::from_node(&node)?;
        Ok(crate::geometry::Geometry { node, info })
    }

    /// Create a node. `name` must start with a network category, e.g, "Object/geo", "Sop/box",
    /// in operator namespace was used, the full name may look like this: namespace::Object/mynode
    /// If you need more creating options, see the [`node_builder`] API.
    /// New node will *not* be cooked.
    pub fn create_node(&self, name: impl AsRef<str>) -> Result<HoudiniNode> {
        self.create_node_with(name.as_ref(), None, None, false)
    }

    /// A builder pattern for creating a node with more options.
    pub fn node_builder(&self, node_name: impl Into<String>) -> NodeBuilder {
        NodeBuilder {
            session: self,
            name: node_name.into(),
            label: None,
            parent: None,
            cook: false,
        }
    }

    // Internal function for creating nodes
    pub(crate) fn create_node_with<P>(
        &self,
        name: &str,
        parent: P,
        label: Option<&str>,
        cook: bool,
    ) -> Result<HoudiniNode>
    where
        P: Into<Option<NodeHandle>>,
    {
        let parent = parent.into();
        debug!("Creating node instance: {}", name);
        debug_assert!(self.is_valid());
        debug_assert!(
            parent.is_some() || name.contains('/'),
            "Node name must be fully qualified if parent is not specified"
        );
        debug_assert!(
            !(parent.is_some() && name.contains('/')),
            "Cannot use fully qualified node name with parent"
        );
        let name = CString::new(name)?;
        let label = label.map(CString::new).transpose()?;
        let id = crate::ffi::create_node(&name, label.as_deref(), self, parent, cook)?;
        HoudiniNode::new(self.clone(), NodeHandle(id), None)
    }

    /// Delete the node from the session. See also [`HoudiniNode::delete`]
    pub fn delete_node<H: Into<NodeHandle>>(&self, node: H) -> Result<()> {
        crate::ffi::delete_node(node.into(), self)
    }

    /// Find a node given an absolute path. To find a child node, pass the `parent` node
    /// or use [`HoudiniNode::find_child_by_path`]
    pub fn get_node_from_path(
        &self,
        path: impl AsRef<str>,
        parent: impl Into<Option<NodeHandle>>,
    ) -> Result<Option<HoudiniNode>> {
        debug_assert!(self.is_valid());
        debug!("Searching node at path: {}", path.as_ref());
        let path = CString::new(path.as_ref())?;
        match crate::ffi::get_node_from_path(self, parent.into(), &path) {
            Ok(handle) => Ok(NodeHandle(handle).to_node(self).ok()),
            Err(HapiError {
                kind: Kind::Hapi(HapiResult::InvalidArgument),
                ..
            }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Find a parameter by its absolute path
    pub fn find_parameter_from_path(&self, path: impl AsRef<str>) -> Result<Option<Parameter>> {
        debug_assert!(self.is_valid());
        debug!("Searching parameter at path: {}", path.as_ref());
        let Some((path, parm)) = path.as_ref().rsplit_once('/') else {
            return Ok(None)
        };
        let Some(node) = self.get_node_from_path(path, None)? else {
            debug!("Node {} not found", path);
            return Ok(None)
        };
        Ok(node.parameter(parm).ok())
    }

    /// Returns a manager (root) node such as OBJ, TOP, CHOP, etc
    pub fn get_manager_node(&self, manager: ManagerType) -> Result<ManagerNode> {
        debug_assert!(self.is_valid());
        debug!("Getting Manager node of type: {:?}", manager);
        let node_type = NodeType::from(manager);
        let handle = crate::ffi::get_manager_node(self, node_type)?;
        Ok(ManagerNode {
            session: self.clone(),
            handle: NodeHandle(handle),
            node_type: manager,
        })
    }

    /// Save current session to hip file
    pub fn save_hip(&self, name: &str, lock_nodes: bool) -> Result<()> {
        debug!("Saving hip file: {}", name);
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        crate::ffi::save_hip(self, &name, lock_nodes)
    }

    /// Load a hip file into current session
    pub fn load_hip(&self, path: impl AsRef<Path>, cook: bool) -> Result<()> {
        debug!("Loading hip file: {:?}", path.as_ref());
        debug_assert!(self.is_valid());
        let name = path_to_cstring(path)?;
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
    pub fn load_asset_file(&self, file: impl AsRef<Path>) -> Result<AssetLibrary> {
        debug_assert!(self.is_valid());
        AssetLibrary::from_file(self.clone(), file)
    }

    /// Interrupt session cooking
    pub fn interrupt(&self) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::interrupt(self)
    }

    /// Get session state of a requested [`crate::enums::StatusType`]
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

    /// Explicit check if the session is valid. Many APIs do this check in the debug build.
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
        debug!("Cooking session..");
        debug_assert!(self.is_valid());
        if self.inner.options.threaded {
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
        self.inner.lock.lock()
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
        debug!("Start rendering COP to image.");
        let cop_node = cop_node.into();
        debug_assert!(cop_node.is_valid(self)?);
        crate::ffi::render_cop_to_image(self, cop_node)?;
        crate::material::extract_image_to_file(self, cop_node, image_planes, path)
    }

    /// Render a COP node to a memory buffer
    pub fn render_cop_to_memory(
        &self,
        cop_node: impl Into<NodeHandle>,
        buffer: &mut Vec<u8>,
        image_planes: impl AsRef<str>,
        format: impl AsRef<str>,
    ) -> Result<()> {
        debug!("Start rendering COP to memory.");
        let cop_node = cop_node.into();
        debug_assert!(cop_node.is_valid(self)?);
        crate::ffi::render_cop_to_image(self, cop_node)?;
        crate::material::extract_image_to_memory(self, cop_node, buffer, image_planes, format)
    }

    pub fn get_supported_image_formats(&self) -> Result<Vec<ImageFileFormat<'_>>> {
        debug_assert!(self.is_valid());
        crate::ffi::get_supported_image_file_formats(self).map(|v| {
            v.into_iter()
                .map(|inner| ImageFileFormat {
                    inner,
                    session: self,
                })
                .collect()
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            debug!("Dropping session");
            if self.is_valid() {
                if self.inner.options.cleanup {
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
            } else {
                // The server should automatically delete the pipe file when closed successfully,
                // but we could try a cleanup just in case.
                warn!("Session is invalid!");
                if let ConnectionType::ThriftPipe(f) = &self.inner.connection {
                    let _ = std::fs::remove_file(f);
                }
            }
        }
    }
}

fn path_to_cstring(path: impl AsRef<Path>) -> Result<CString> {
    let s = path.as_ref().as_os_str().to_string_lossy().to_string();
    Ok(CString::new(s)?)
}

/// Connect to the engine process via a pipe file.
/// If `timeout` is Some, function will try to connect to
/// the server multiple times every 100ms until `timeout` is reached.
pub fn connect_to_pipe(
    pipe: impl AsRef<Path>,
    options: Option<&SessionOptions>,
    timeout: Option<Duration>,
) -> Result<Session> {
    debug!("Connecting to Thrift session: {:?}", pipe.as_ref());
    let c_str = path_to_cstring(&pipe)?;
    let pipe = pipe.as_ref().as_os_str().to_os_string();
    let timeout = timeout.unwrap_or_default();
    let mut waited = Duration::from_secs(0);
    let wait_ms = Duration::from_millis(100);
    let handle = loop {
        let mut last_error = None;
        debug!("Trying to connect to pipe server");
        match crate::ffi::new_thrift_piped_session(&c_str) {
            Ok(handle) => break handle,
            Err(e) => {
                last_error.replace(e);
                std::thread::sleep(wait_ms);
                waited += wait_ms;
            }
        }
        if waited > timeout {
            // last_error is guarantied to be Some().
            return Err(last_error.unwrap()).context("Connection timeout");
        }
    };
    let connection = ConnectionType::ThriftPipe(pipe);
    let session = Session::new(handle, connection, options.cloned().unwrap_or_default());
    session.initialize()?;
    Ok(session)
}

/// Connect to the engine process via a Unix socket
pub fn connect_to_socket(
    addr: std::net::SocketAddrV4,
    options: Option<&SessionOptions>,
) -> Result<Session> {
    debug!("Connecting to socket server: {:?}", addr);
    let host = CString::new(addr.ip().to_string()).expect("SocketAddr->CString");
    let handle = crate::ffi::new_thrift_socket_session(addr.port() as i32, &host)?;
    let connection = ConnectionType::ThriftSocket(addr);
    let session = Session::new(handle, connection, options.cloned().unwrap_or_default());
    session.initialize()?;
    Ok(session)
}

/// Create in-process session
pub fn new_in_process(options: Option<&SessionOptions>) -> Result<Session> {
    debug!("Creating new in-process session");
    let handle = crate::ffi::create_inprocess_session()?;
    let connection = ConnectionType::InProcess;
    let session = Session::new(handle, connection, options.cloned().unwrap_or_default());
    session.initialize()?;
    Ok(session)
}

/// Session options passed to session create functions like [`connect_to_pipe`]
#[derive(Clone, Debug)]
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
    pub env_variables: Option<Vec<(String, String)>>,
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
            env_variables: None,
            otl_path: None,
            dso_path: None,
            img_dso_path: None,
            aud_dso_path: None,
        }
    }
}

#[derive(Default)]
/// A build for SessionOptions.
pub struct SessionOptionsBuilder {
    cook_opt: CookOptions,
    threaded: bool,
    cleanup: bool,
    ignore_already_init: bool,
    env_variables: Option<Vec<(String, String)>>,
    env_files: Option<CString>,
    otl_path: Option<CString>,
    dso_path: Option<CString>,
    img_dso_path: Option<CString>,
    aud_dso_path: Option<CString>,
}

impl SessionOptionsBuilder {
    /// A list of Houdini environment file the Engine will load environment from.
    pub fn houdini_env_files<I>(mut self, files: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let paths = utils::join_paths(files);
        self.env_files
            .replace(CString::new(paths).expect("Zero byte"));
        self
    }

    /// Set the server environment variables. See also [`Session::set_server_var`].
    /// The difference is this method writes out a temp file with the variables and
    /// implicitly pass it to the engine (as if [`Self::houdini_env_files`] was used.
    pub fn env_variables<I, K, V>(mut self, variables: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.env_variables.replace(
            variables
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// Add search paths for the Engine to find HDAs.
    pub fn otl_search_paths<I>(mut self, paths: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let paths = utils::join_paths(paths);
        self.otl_path
            .replace(CString::new(paths).expect("Zero byte"));
        self
    }

    /// Add search paths for the Engine to find DSO plugins.
    pub fn dso_search_paths<P>(mut self, paths: P) -> Self
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        let paths = utils::join_paths(paths);
        self.dso_path
            .replace(CString::new(paths).expect("Zero byte"));
        self
    }

    /// Add search paths for the Engine to find image plugins.
    pub fn image_search_paths<P>(mut self, paths: P) -> Self
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        let paths = utils::join_paths(paths);
        self.img_dso_path
            .replace(CString::new(paths).expect("Zero byte"));
        self
    }

    /// Add search paths for the Engine to find audio files.
    pub fn audio_search_paths<P>(mut self, paths: P) -> Self
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        let paths = utils::join_paths(paths);
        self.aud_dso_path
            .replace(CString::new(paths).expect("Zero byte"));
        self
    }

    /// Do not error when connecting to a server process which has a session already initialized.
    pub fn ignore_already_init(mut self, ignore: bool) -> Self {
        self.ignore_already_init = ignore;
        self
    }

    /// Pass session [`CookOptions`]
    pub fn cook_options(mut self, options: CookOptions) -> Self {
        self.cook_opt = options;
        self
    }

    /// Makes the server operate in threaded mode. See the official docs for more info.
    pub fn threaded(mut self, threaded: bool) -> Self {
        self.threaded = threaded;
        self
    }

    /// Cleanup the server session when the last connection drops.
    pub fn cleanup_on_close(mut self, cleanup: bool) -> Self {
        self.cleanup = cleanup;
        self
    }

    /// Consume the builder and return the result.
    pub fn build(mut self) -> SessionOptions {
        self.write_temp_env_file();
        SessionOptions {
            cook_opt: self.cook_opt,
            threaded: self.threaded,
            cleanup: self.cleanup,
            ignore_already_init: self.cleanup,
            env_files: self.env_files,
            env_variables: self.env_variables,
            otl_path: self.otl_path,
            dso_path: self.dso_path,
            img_dso_path: self.img_dso_path,
            aud_dso_path: self.aud_dso_path,
        }
    }
    // Helper function for Self::env_variables
    fn write_temp_env_file(&mut self) {
        use std::io::Write;

        if let Some(ref env) = self.env_variables {
            let mut file = tempfile::Builder::new()
                .suffix("_hars.env")
                .tempfile()
                .expect("tempfile");
            for (k, v) in env.iter() {
                writeln!(file, "{}={}", k, v).expect("write to .env file");
            }
            let (_, tmp_file) = file.keep().expect("persistent tempfile");
            let tmp_file = CString::new(tmp_file.to_string_lossy().to_string()).expect("null byte");

            if let Some(old) = &mut self.env_files {
                let mut bytes = old.as_bytes_with_nul().to_vec();
                bytes.extend(tmp_file.into_bytes_with_nul());
                self.env_files
                    // SAFETY: the bytes vec was obtained from the two CString's above.
                    .replace(unsafe { CString::from_vec_with_nul_unchecked(bytes) });
            } else {
                self.env_files.replace(tmp_file);
            }
        }
    }
}

impl SessionOptions {
    /// Create a [`SessionOptionsBuilder`]. Same as [`SessionOptionsBuilder::default()`].
    pub fn builder() -> SessionOptionsBuilder {
        SessionOptionsBuilder::default()
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
    path: impl AsRef<Path>,
    auto_close: bool,
    timeout: f32,
    verbosity: StatusVerbosity,
    log_file: Option<&str>,
) -> Result<u32> {
    debug!("Starting named pipe server: {:?}", path.as_ref());
    let opts = crate::ffi::raw::HAPI_ThriftServerOptions {
        autoClose: auto_close as i8,
        timeoutMs: timeout,
        verbosity,
    };
    let log_file = log_file.map(CString::new).transpose()?;
    let c_str = path_to_cstring(path)?;
    crate::ffi::start_thrift_pipe_server(&c_str, &opts, log_file.as_deref())
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
    let log_file = log_file.map(CString::new).transpose()?;
    crate::ffi::start_thrift_socket_server(port as i32, &opts, log_file.as_deref())
}

/// Start a interactive Houdini session with engine server embedded.
pub fn start_houdini_server(
    pipe_name: impl AsRef<str>,
    houdini_executable: impl AsRef<Path>,
) -> Result<Child> {
    std::process::Command::new(houdini_executable.as_ref())
        .arg(format!("-hess=pipe:{}", pipe_name.as_ref()))
        .spawn()
        .map_err(HapiError::from)
}

/// A quick drop-in session, useful for on-off jobs
/// It starts a single-threaded pipe server and initialize a session with default options
pub fn quick_session(options: Option<&SessionOptions>) -> Result<Session> {
    let file = tempfile::Builder::new()
        .suffix("-hars.pipe")
        .tempfile()
        .expect("new temp file");
    let (_, file) = file.keep().expect("persistent temp file");
    start_engine_pipe_server(&file, true, 4000.0, StatusVerbosity::Statusverbosity1, None)?;
    connect_to_pipe(file, options, None)
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::session::*;
    use once_cell::sync::Lazy;
    use std::default::Default;

    static SESSION: Lazy<Session> = Lazy::new(|| {
        env_logger::init();
        let session = quick_session(None).expect("Could not create test session");
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
        let opt = super::SessionOptions::builder()
            .dso_search_paths(["/path/one", "/path/two"])
            .otl_search_paths(["/path/thee", "/path/four"])
            .build();
        let ses = super::quick_session(Some(&opt)).unwrap();
        assert!(matches!(
            ses.connection_type(),
            ConnectionType::ThriftPipe(_)
        ));
        assert!(ses.is_initialized());
        assert!(ses.is_valid());
        assert!(ses.cleanup().is_ok());
        assert!(!ses.is_initialized());
    }

    #[test]
    fn session_time() {
        // For some reason, this test randomly fails when using shared session
        let session = quick_session(None).expect("Could not start session");
        let _lock = session.lock();
        let opt = TimelineOptions::default().with_end_time(5.5);
        assert!(session.set_timeline_options(opt.clone()).is_ok());
        let opt2 = session.get_timeline_options().expect("timeline_options");
        assert!(opt.end_time().eq(&opt2.end_time()));
        session.set_time(4.12).expect("set_time");
        assert!(matches!(session.cook(), Ok(CookResult::Succeeded)));
        assert_eq!(session.get_time().expect("get_time"), 4.12);
    }

    #[test]
    fn server_variables() {
        // Starting a new separate session because getting/setting env variables from multiple
        // clients ( threads ) breaks the server
        let session = super::quick_session(None).expect("Could not start session");
        session.set_server_var::<str>("FOO", "foo_string").unwrap();
        assert_eq!(session.get_server_var::<str>("FOO").unwrap(), "foo_string");
        session.set_server_var::<i32>("BAR", &123).unwrap();
        assert_eq!(session.get_server_var::<i32>("BAR").unwrap(), 123);
        assert!(!session.get_server_variables().unwrap().is_empty());
    }

    #[test]
    fn create_node_async() {
        use crate::ffi::raw::{NodeFlags, NodeType};
        let opt = SessionOptions::builder().threaded(true).build();
        let session = super::quick_session(Some(&opt)).unwrap();
        session
            .load_asset_file("otls/sesi/SideFX_spaceship.hda")
            .unwrap();
        let node = session.create_node("Object/spaceship").unwrap();
        assert_eq!(
            node.cook_count(NodeType::None, NodeFlags::None, true)
                .unwrap(),
            0
        );
        node.cook().unwrap(); // in threaded mode always successful
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

    #[test]
    fn manager_nodes() {
        with_session(|session| {
            session.get_manager_node(ManagerType::Obj).unwrap();
            session.get_manager_node(ManagerType::Chop).unwrap();
            session.get_manager_node(ManagerType::Cop).unwrap();
            session.get_manager_node(ManagerType::Rop).unwrap();
            session.get_manager_node(ManagerType::Top).unwrap();
        })
    }
}
