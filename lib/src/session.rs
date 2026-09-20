//! Session is responsible for communicating with HAPI
//!
//! The Engine [promises](https://www.sidefx.com/docs/hengine/_h_a_p_i__sessions.html#HAPI_Sessions_Multithreading)
//! to be thread-safe when accessing a single `Session` from multiple threads.
//! `hapi-rs` relies on this promise and the [Session] struct holds only an `Arc` pointer to the session,
//! and *does not* protect the session with Mutex, although there is a [`ReentrantMutex`]
//! private member which is used internally in a few cases where API calls must be sequential.
//!
//! When the last instance of the `Session` is about to get dropped, it'll be cleaned up
//! (if [`SessionOptions::cleanup`] was set) and automatically closed.
//!
//! The Engine process (pipe, socket, or shared memory) can be auto-terminated as well if told so when starting
//! the server. See [`crate::server::start_engine_server`] together with the transport helpers
//! [`crate::server::connect_to_pipe_server`], [`crate::server::connect_to_socket_server`], and
//! [`crate::server::connect_to_memory_server`].
//!
//! Helper constructors terminate the server by default. This is useful for quick one-off jobs.
//!
use log::{debug, error};
use parking_lot::{Mutex, ReentrantMutex};
use std::fmt::Debug;
use std::path::PathBuf;
use std::{
    ffi::CString,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub use crate::{
    asset::AssetLibrary,
    errors::*,
    ffi::{
        CompositorOptions, CookOptions, ImageFileFormat, ImageInfo, SessionInfo, SessionSyncInfo,
        ThriftServerOptions, TimelineOptions, Viewport, enums::*,
    },
    node::{HoudiniNode, ManagerNode, ManagerType, NodeHandle, NodeType, Transform},
    parameter::Parameter,
    server::ServerOptions,
    stringhandle::StringArray,
};

/// State returned by `HAPI_GetStatus` for `HAPI_STATUS_COOK_STATE`.
pub type SessionState = State;
/// Houdini license type currently checked out by the session.
pub type LicenseType = raw::License;

use crate::cop::CopImageDescription;
use crate::ffi::CameraInfo;
use crate::stringhandle::StringHandle;
use crate::{ffi::raw, utils};

static IN_PROCESS_RUNTIME_SHUT_DOWN: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub(crate) enum ServerConnection {
    Borrowed {
        reported_pid: Option<u32>,
    },
    OwnedHars {
        pid: u32,
        auto_close: bool,
        pipe_path: Option<PathBuf>,
    },
    InProcess,
}

impl ServerConnection {
    fn pid(&self) -> Option<u32> {
        match self {
            Self::Borrowed { reported_pid } => *reported_pid,
            Self::OwnedHars { pid, .. } => Some(*pid),
            Self::InProcess => None,
        }
    }

    fn finish(&self, construction_failed: bool) -> Result<()> {
        match self {
            Self::OwnedHars {
                pid,
                auto_close,
                pipe_path,
            } => crate::server::finish_owned_server(
                *pid,
                *auto_close,
                construction_failed,
                pipe_path.as_deref(),
            ),
            Self::Borrowed { .. } | Self::InProcess => Ok(()),
        }
    }
}

#[derive(Debug)]
struct LifecycleState {
    closed: bool,
    server: ServerConnection,
}

trait LifecycleBackend {
    fn is_valid(&mut self, handle: &raw::HAPI_Session) -> bool;
    fn is_initialized(&mut self, handle: &raw::HAPI_Session) -> bool;
    fn cleanup(&mut self, handle: &raw::HAPI_Session) -> Result<()>;
    fn shutdown(&mut self, handle: &raw::HAPI_Session) -> Result<()>;
    fn close(&mut self, handle: &raw::HAPI_Session) -> Result<()>;
    fn finish_server(&mut self, server: &ServerConnection, construction_failed: bool)
    -> Result<()>;
    fn mark_in_process_shutdown(&mut self);
}

struct HapiLifecycleBackend;

impl LifecycleBackend for HapiLifecycleBackend {
    fn is_valid(&mut self, handle: &raw::HAPI_Session) -> bool {
        crate::ffi::is_raw_session_valid(handle)
    }

    fn is_initialized(&mut self, handle: &raw::HAPI_Session) -> bool {
        crate::ffi::is_raw_session_initialized(handle)
    }

    fn cleanup(&mut self, handle: &raw::HAPI_Session) -> Result<()> {
        crate::ffi::cleanup_raw_session(handle)
    }

    fn shutdown(&mut self, handle: &raw::HAPI_Session) -> Result<()> {
        crate::ffi::shutdown_raw_session(handle)
    }

    fn close(&mut self, handle: &raw::HAPI_Session) -> Result<()> {
        crate::ffi::close_raw_session(handle)
    }

    fn finish_server(
        &mut self,
        server: &ServerConnection,
        construction_failed: bool,
    ) -> Result<()> {
        server.finish(construction_failed)
    }

    fn mark_in_process_shutdown(&mut self) {
        IN_PROCESS_RUNTIME_SHUT_DOWN.store(true, Ordering::Release);
    }
}

fn lifecycle_errors(errors: Vec<String>, operation: &str) -> Result<()> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(HapiError::Internal(format!(
            "{operation} encountered errors: {}",
            errors.join("; ")
        )))
    }
}

/// Builder struct for [`Session::node_builder`] API
pub struct NodeBuilder<'s> {
    session: &'s Session,
    name: String,
    label: Option<String>,
    parent: Option<NodeHandle>,
    cook: bool,
}

impl NodeBuilder<'_> {
    /// Give new node a label
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Create new node as child of a parent node.
    #[must_use]
    pub fn with_parent<H: AsRef<NodeHandle>>(mut self, parent: H) -> Self {
        self.parent.replace(*parent.as_ref());
        self
    }

    /// Cook node after creation.
    #[must_use]
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
        let handle = crate::ffi::get_server_env_str(session, &key)?;
        crate::stringhandle::get_string(handle, session)
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
        let val = utils::path_to_cstring(val)?;
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
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CookResult {
    /// Cooking completed without cook or fatal errors.
    Succeeded,
    /// Some nodes cooked with errors
    CookErrors(String),
    /// One or more nodes could not cook - should abort cooking
    FatalErrors(String),
}

impl CookResult {
    /// Convenient method for cook result message if any
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Succeeded => None,
            Self::CookErrors(msg) | Self::FatalErrors(msg) => Some(msg.as_str()),
        }
    }
}

/// By which means the session communicates with the server.
#[derive(Debug)]
pub(crate) struct SessionInner {
    pub(crate) handle: raw::HAPI_Session,
    pub(crate) options: SessionOptions,
    pub(crate) lock: ReentrantMutex<()>,
    lifecycle: Mutex<LifecycleState>,
}

/// An initialized connection to a Houdini Engine runtime.
///
/// Clones share one underlying HAPI session. The connection remains open until
/// [`Self::close`] is called or the last clone and all objects retaining a clone
/// are dropped. Final teardown is performed exactly once.
///
/// `Session` is [`Send`] and [`Sync`]. HARC serializes individual calls made to
/// one Thrift session; callers must still coordinate multi-call operations for
/// which ordering matters.
///
/// Dropping or closing a session always calls `HAPI_CloseSession`. The optional,
/// potentially expensive `HAPI_Cleanup` step is controlled by
/// [`SessionOptions::cleanup`] and is disabled by default.
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

/// A live HAPI connection that has not yet been initialized.
///
/// Values are returned by the `connect_to_*` helpers in [`crate::server`]. Call
/// [`Self::initialize`] to run `HAPI_Initialize` and obtain a [`Session`]. If
/// this value is dropped or initialization fails, its native connection is
/// closed automatically.
#[derive(Debug)]
pub struct UninitializedSession {
    pub(crate) session_handle: Option<raw::HAPI_Session>,
    pub(crate) server_options: Option<ServerOptions>,
    pub(crate) server: ServerConnection,
}

impl UninitializedSession {
    /// Initialize the Houdini Engine runtime for this connection.
    ///
    /// On success, ownership of the native handle transfers to the returned
    /// [`Session`]. On failure, the connection is closed during rollback.
    pub fn initialize(mut self, session_options: SessionOptions) -> Result<Session> {
        debug!("Initializing session");
        let handle = self
            .session_handle
            .expect("uninitialized session handle was already consumed");
        crate::ffi::initialize_session(handle, &session_options)
            .with_context(|| "Calling initialize_session")?;

        self.session_handle.take();
        let server = std::mem::replace(
            &mut self.server,
            ServerConnection::Borrowed { reported_pid: None },
        );
        Ok(Session {
            inner: Arc::new(SessionInner {
                handle,
                options: session_options,
                lock: ReentrantMutex::new(()),
                lifecycle: Mutex::new(LifecycleState {
                    closed: false,
                    server,
                }),
            }),
        })
    }

    pub(crate) fn mark_owned_server(&mut self, pid: u32) {
        let (auto_close, pipe_path) =
            self.server_options
                .as_ref()
                .map_or((true, None), |options| {
                    let pipe_path = match &options.thrift_transport {
                        crate::server::ThriftTransport::Pipe(transport) => {
                            Some(transport.pipe_path.clone())
                        }
                        crate::server::ThriftTransport::SharedMemory(_)
                        | crate::server::ThriftTransport::Socket(_) => None,
                    };
                    (options.auto_close, pipe_path)
                });
        self.server = ServerConnection::OwnedHars {
            pid,
            auto_close,
            pipe_path,
        };
    }
}

impl Drop for UninitializedSession {
    fn drop(&mut self) {
        let handle = self.session_handle.take();
        if let Err(error) =
            rollback_uninitialized_with(handle.as_ref(), &self.server, &mut HapiLifecycleBackend)
        {
            error!("Uninitialized session rollback failed: {error}");
        }
    }
}

fn rollback_uninitialized_with<B: LifecycleBackend>(
    handle: Option<&raw::HAPI_Session>,
    server: &ServerConnection,
    backend: &mut B,
) -> Result<()> {
    let mut errors = Vec::new();
    if let Some(handle) = handle
        && backend.is_valid(handle)
    {
        if backend.is_initialized(handle)
            && let Err(error) = backend.cleanup(handle)
        {
            errors.push(error.to_string());
        }
        if matches!(server, ServerConnection::InProcess)
            && let Err(error) = backend.shutdown(handle)
        {
            errors.push(error.to_string());
        }
        if let Err(error) = backend.close(handle) {
            errors.push(error.to_string());
        }
    }
    if matches!(server, ServerConnection::InProcess) {
        backend.mark_in_process_shutdown();
    }
    if let Err(error) = backend.finish_server(server, true) {
        errors.push(error.to_string());
    }
    lifecycle_errors(errors, "Uninitialized session rollback")
}

impl Session {
    /// Return [`SessionType`] current session is initialized with.
    #[must_use]
    pub fn session_type(&self) -> SessionType {
        self.inner.handle.type_
    }

    /// Return the server PID when it was supplied by the caller or the server
    /// was started by this crate. In-process sessions return `None`.
    #[must_use]
    pub fn server_pid(&self) -> Option<u32> {
        self.inner.lifecycle.lock().server.pid()
    }

    #[inline]
    pub(crate) fn ptr(&self) -> *const raw::HAPI_Session {
        &raw const self.inner.handle
    }

    /// Set environment variable on the server. This is set AFTER the server has started.
    /// For variables set before the server starts, use [`ServerOptions::with_env_variables`].
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
        debug!("Querying server variable {key}");
        T::get_value(self, key)
    }

    /// Retrieve all server variables as contiguous string array.
    /// Iterating over the array will yield strings like "TEST=177".
    pub fn get_server_variables(&self) -> Result<StringArray> {
        debug_assert!(self.is_valid());
        debug!("Querying all server variables");
        let count = crate::ffi::get_server_env_var_count(self)?;
        let handles = crate::ffi::get_server_env_var_list(self, count)?;
        crate::stringhandle::get_string_array(&handles, self).context("Calling get_string_array")
    }

    /// Retrieve string data given a handle.
    pub fn get_string(&self, handle: StringHandle) -> Result<String> {
        crate::stringhandle::get_string(handle, self)
    }

    /// Retrieve multiple strings in batch mode.
    pub fn get_string_batch(&self, handles: &[StringHandle]) -> Result<StringArray> {
        crate::stringhandle::get_string_array(handles, self)
    }

    /// Push a custom string to the server and return a handle to it.
    pub fn set_custom_string(&self, string: impl AsRef<str>) -> Result<StringHandle> {
        debug_assert!(self.is_valid());
        debug!("Setting custom string: {}", string.as_ref());
        let string = CString::new(string.as_ref())?;
        crate::ffi::set_custom_string(self, &string)
    }

    /// Remove a custom string from the server.
    pub fn remove_custom_string(&self, handle: StringHandle) -> Result<()> {
        debug_assert!(self.is_valid());
        debug!("Removing custom string: {handle:?}");
        crate::ffi::remove_custom_string(self, handle)
    }

    /// Close the shared underlying session.
    ///
    /// This operation is idempotent. All clones become closed after this call.
    pub fn close(&self) -> Result<()> {
        self.inner.finalize()
    }

    /// Close the shared underlying session.
    #[deprecated(note = "use Session::close; HAPI cleanup alone does not close a session")]
    pub fn cleanup(self) -> Result<()> {
        self.close()
    }

    /// Create an input geometry node which can accept modifications
    pub fn create_input_node(
        &self,
        name: &str,
        parent: Option<NodeHandle>,
    ) -> Result<crate::geometry::Geometry> {
        debug!("Creating input node: {name}");
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        let id = crate::ffi::create_input_node(self, &name, parent)?;
        let node = HoudiniNode::new(self.clone(), NodeHandle(id), None)?;
        crate::ffi::cook_node(&node, None).with_context(|| "Cooking input node")?;
        let info =
            crate::geometry::GeoInfo::from_node(&node).with_context(|| "Getting geometry info")?;
        Ok(crate::geometry::Geometry { node, info })
    }

    /// Create an input geometry node with [`PartType`] set to `Curve`
    pub fn create_input_curve_node(
        &self,
        name: &str,
        parent: Option<NodeHandle>,
    ) -> Result<crate::geometry::Geometry> {
        debug!("Creating input curve node: {name}");
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        let id = crate::ffi::create_input_curve_node(self, &name, parent)?;
        let node = HoudiniNode::new(self.clone(), NodeHandle(id), None)?;
        let info = crate::geometry::GeoInfo::from_node(&node)?;
        Ok(crate::geometry::Geometry { node, info })
    }

    /// Create a node. `name` must start with a network category, e.g, "Object/geo", "Sop/box",
    /// in operator namespace was used, the full name may look like this: `namespace::Object/mynode`
    /// If you need more creating options, see the [`Session::node_builder`] API.
    /// New node will *not* be cooked.
    pub fn create_node(&self, name: impl AsRef<str>) -> Result<HoudiniNode> {
        self.create_node_with(name.as_ref(), None, None, false)
    }

    /// A builder pattern for creating a node with more options.
    pub fn node_builder(&self, node_name: impl Into<String>) -> NodeBuilder<'_> {
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
        debug!("Creating node instance for op: {name}, with parent: {parent:?}");
        debug_assert!(self.is_valid());
        debug_assert!(
            parent.is_some() || name.contains('/'),
            "Node name must be fully qualified if parent node is not specified"
        );
        debug_assert!(
            !(parent.is_some() && name.contains('/')),
            "Cannot use fully qualified node name with parent node"
        );
        let name = CString::new(name)?;
        let label = label.map(CString::new).transpose()?;
        let node_id = crate::ffi::create_node(&name, label.as_deref(), self, parent, cook)?;
        if self.inner.options.threaded {
            // In async cooking mode, cook() always returns a CookResult::Success, we need to check CookResult for errors
            if let CookResult::FatalErrors(message) = self.cook()? {
                return Err(HapiError::Hapi {
                    result_code: HapiResultCode(HapiResult::Failure),
                    server_message: Some(message),
                    contexts: Vec::new(),
                });
            }
        }
        HoudiniNode::new(self.clone(), NodeHandle(node_id), None)
    }

    /// Delete the node from the session. See also [`HoudiniNode::delete`]
    pub fn delete_node<H: Into<NodeHandle>>(&self, node: H) -> Result<()> {
        let node = node.into();
        debug!(
            "Deleting node {}",
            node.path(self)
                .unwrap_or_else(|_| "Could not get path".to_owned())
        );
        crate::ffi::delete_node(node, self)
    }

    /// Find a node given an absolute path. To find a child node, pass the `parent` node
    /// or use [`HoudiniNode::find_child_node`]
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
            Err(HapiError::Hapi { result_code, .. })
                if matches!(result_code.0, HapiResult::InvalidArgument) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Find a parameter by path, absolute or relative to a start node.
    pub fn find_parameter_from_path(
        &self,
        path: impl AsRef<str>,
        start: impl Into<Option<NodeHandle>>,
    ) -> Result<Option<Parameter>> {
        debug_assert!(self.is_valid());
        debug!("Searching parameter at path: {}", path.as_ref());
        let Some((path, parm)) = path.as_ref().rsplit_once('/') else {
            return Ok(None);
        };
        let Some(node) = self.get_node_from_path(path, start)? else {
            debug!("Node {path} not found");
            return Ok(None);
        };
        Ok(node.parameter(parm).ok())
    }

    /// Returns a manager (root) node such as OBJ, TOP, CHOP, etc
    pub fn get_manager_node(&self, manager: ManagerType) -> Result<ManagerNode> {
        debug_assert!(self.is_valid());
        debug!("Getting Manager node of type: {manager:?}");
        let node_type = NodeType::from(manager);
        let handle = crate::ffi::get_manager_node(self, node_type)?;
        Ok(ManagerNode {
            session: self.clone(),
            handle: NodeHandle(handle),
            node_type: manager,
        })
    }

    /// Return a list of transforms for all object nodes under a given parent node.
    pub fn get_composed_object_transform(
        &self,
        parent: impl AsRef<NodeHandle>,
        rst_order: RSTOrder,
    ) -> Result<Vec<Transform>> {
        debug_assert!(self.is_valid());
        crate::ffi::get_composed_object_transforms(self, *parent.as_ref(), rst_order)
            .map(|transforms| transforms.into_iter().map(Transform).collect())
    }

    /// Save current session to hip file
    pub fn save_hip(&self, path: impl AsRef<Path>, lock_nodes: bool) -> Result<()> {
        debug!("Saving hip file: {}", path.as_ref().display());
        debug_assert!(self.is_valid());
        let path = utils::path_to_cstring(path)?;
        crate::ffi::save_hip(self, &path, lock_nodes)
    }

    /// Load a hip file into current session
    pub fn load_hip(&self, path: impl AsRef<Path>, cook: bool) -> Result<()> {
        debug!("Loading hip file: {}", path.as_ref().display());
        debug_assert!(self.is_valid());
        let path = utils::path_to_cstring(path)?;
        crate::ffi::load_hip(self, &path, cook)
    }

    /// Merge a hip file into current session
    pub fn merge_hip(&self, name: &str, cook: bool) -> Result<i32> {
        debug!("Merging hip file: {name}");
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        crate::ffi::merge_hip(self, &name, cook)
    }

    /// Get node ids created by merging [`Session::merge_hip`] a hip file.
    pub fn get_hip_file_nodes(&self, hip_id: i32) -> Result<Vec<NodeHandle>> {
        crate::ffi::get_hipfile_node_ids(self, hip_id)
            .map(|handles| handles.into_iter().map(NodeHandle).collect())
    }

    /// Load an HDA file into current session
    pub fn load_asset_file(&self, file: impl AsRef<Path>) -> Result<AssetLibrary> {
        debug_assert!(self.is_valid());
        AssetLibrary::from_file(self.clone(), file)
    }

    /// Returns a list of loaded asset libraries including Houdini's default.
    pub fn get_loaded_asset_libraries(&self) -> Result<Vec<AssetLibrary>> {
        debug_assert!(self.is_valid());

        crate::ffi::get_asset_library_ids(self)?
            .into_iter()
            .map(|library_id| {
                crate::ffi::get_asset_library_file_path(self, library_id).map(|path| AssetLibrary {
                    lib_id: library_id,
                    session: self.clone(),
                    file: Some(PathBuf::from(path)),
                })
            })
            .collect()
    }

    /// Interrupt session cooking
    pub fn interrupt(&self) -> Result<()> {
        debug_assert!(self.is_valid());
        debug!("Interrupting session cooking");
        crate::ffi::interrupt(self)
    }

    // Uncertain if this API makes sense.
    #[doc(hidden)]
    #[allow(unused)]
    pub(crate) fn get_call_result_status(&self) -> Result<HapiResult> {
        debug_assert!(self.is_valid());
        let status = crate::ffi::get_status_code(self, StatusType::CallResult)?;
        Ok(unsafe { std::mem::transmute::<i32, HapiResult>(status) })
    }

    /// Get session state when the server is in threaded mode.
    pub fn get_cook_state_status(&self) -> Result<SessionState> {
        debug_assert!(self.is_valid());
        crate::ffi::get_cook_state_status(self)
    }

    /// Is session currently cooking. In non-threaded mode always returns false
    pub fn is_cooking(&self) -> Result<bool> {
        debug_assert!(self.is_valid());
        Ok(matches!(
            self.get_cook_state_status()?,
            SessionState::Cooking
        ))
    }

    /// Explicit check if the session is valid. Many APIs do this check in the debug build.
    #[inline]
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.inner.lifecycle.lock().closed && crate::ffi::is_session_valid(self)
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
        debug!("Cooking session..");
        if self.inner.options.threaded {
            loop {
                match self.get_cook_state_status()? {
                    SessionState::Ready => break Ok(CookResult::Succeeded),
                    SessionState::ReadyWithFatalErrors => {
                        self.interrupt()?;
                        let err = self.get_cook_result_string(StatusVerbosity::Errors)?;
                        break Ok(CookResult::FatalErrors(err));
                    }
                    SessionState::ReadyWithCookErrors => {
                        let err = self.get_cook_result_string(StatusVerbosity::Errors)?;
                        break Ok(CookResult::CookErrors(err));
                    }
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
    pub fn get_time(&self) -> Result<f64> {
        debug_assert!(self.is_valid());
        crate::ffi::get_time(self)
    }

    /// Set Houdini time
    pub fn set_time(&self, time: f64) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::set_time(self, time)
    }

    /// Lock the internal reentrant mutex. Should not be used in general, but may be useful
    /// in certain situations when a series of API calls must be done in sequence
    pub fn lock(&self) -> parking_lot::ReentrantMutexGuard<'_, ()> {
        self.inner.lock.lock()
    }

    /// Set Houdini timeline options
    pub fn set_timeline_options(&self, options: &TimelineOptions) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::set_timeline_options(self, &options.0)
    }

    /// Get Houdini timeline options
    pub fn get_timeline_options(&self) -> Result<TimelineOptions> {
        debug_assert!(self.is_valid());
        crate::ffi::get_timeline_options(self).map(TimelineOptions)
    }

    /// Set session to use Houdini time
    pub fn set_use_houdini_time(&self, do_use: bool) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::set_use_houdini_time(self, do_use)
    }

    /// Check if session uses Houdini time
    pub fn get_use_houdini_time(&self) -> Result<bool> {
        debug_assert!(self.is_valid());
        crate::ffi::get_use_houdini_time(self)
    }

    /// Get the viewport(camera) position
    pub fn get_viewport(&self) -> Result<Viewport> {
        debug_assert!(self.is_valid());
        crate::ffi::get_viewport(self).map(Viewport)
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
        crate::ffi::get_session_sync_info(self).map(SessionSyncInfo)
    }

    /// Set session sync info
    pub fn set_sync_info(&self, info: &SessionSyncInfo) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::set_session_sync_info(self, &info.0)
    }

    /// Get license type used by this session
    pub fn get_license_type(&self) -> Result<LicenseType> {
        debug_assert!(self.is_valid());
        crate::ffi::session_get_license_type(self)
    }

    /// Render a COP node to an image file
    pub fn render_cop_to_image(
        &self,
        cop_node: impl Into<NodeHandle>,
        output_name: Option<&str>,
        image_planes: impl AsRef<str>,
        out_image: impl AsRef<Path>,
    ) -> Result<String> {
        let cop_node = cop_node.into();
        let out_image = out_image.as_ref();
        debug!("Start rendering COP to image file {}", out_image.display());
        debug_assert!(cop_node.is_valid(self)?);
        if let Some(output_name) = output_name {
            let output_name = CString::new(output_name)?;
            crate::ffi::render_cop_output_to_image(self, cop_node, &output_name)?;
        } else {
            crate::ffi::render_cop_to_image(self, cop_node)?;
        }
        crate::material::extract_image_to_file(self, cop_node, image_planes, out_image)
    }

    /// Create a new input camera node with the given name/label.
    pub fn create_input_camera_node(
        &self,
        name: &str,
        label: &str,
        parent_node: Option<NodeHandle>,
    ) -> Result<NodeHandle> {
        let name = CString::new(name)?;
        let label = CString::new(label)?;
        crate::ffi::create_input_camera_node(self, parent_node, &name, &label)
    }

    /// Set the camera parameters on an input camera node.
    pub fn set_input_camera_info(&self, node: NodeHandle, info: &CameraInfo) -> Result<()> {
        crate::ffi::set_input_camera_info(node, self, &info.0)
    }

    /// Set the transform on an input camera node.
    pub fn set_input_camera_transform(
        &self,
        node: NodeHandle,
        rst_order: RSTOrder,
        rot_order: XYZOrder,
        transform: &Transform,
    ) -> Result<()> {
        crate::ffi::set_input_camera_transform(node, self, rst_order, rot_order, &transform.0)
    }

    /// Loads some raw image data into a COP node, returning the handle of the created node.
    pub fn create_cop_image(
        &self,
        description: CopImageDescription,
        parent_node: Option<NodeHandle>,
    ) -> Result<NodeHandle> {
        crate::ffi::create_cop_image(
            self,
            parent_node,
            description.width,
            description.height,
            description.packing,
            description.flip_x,
            description.flip_y,
            description.image_data,
        )
    }

    // TODO: consider removing this in favour of ['Material'] helper struct which has this API
    pub fn render_texture_to_image(
        &self,
        node: impl Into<NodeHandle>,
        parm_name: &str,
    ) -> Result<()> {
        debug_assert!(self.is_valid());
        let name = CString::new(parm_name)?;
        let node = node.into();
        let id = crate::ffi::get_parm_id_from_name(&name, node, self)?;
        crate::ffi::render_texture_to_image(self, node, crate::parameter::ParmHandle(id))
    }

    // TODO: consider removing this in favour of ['Material'] helper struct which has this API
    pub fn extract_image_to_file(
        &self,
        node: impl Into<NodeHandle>,
        image_planes: &str,
        path: impl AsRef<Path>,
    ) -> Result<String> {
        crate::material::extract_image_to_file(self, node.into(), image_planes, path)
    }

    // TODO: consider removing this in favour of ['Material'] helper struct which has this API
    pub fn extract_image_to_memory(
        &self,
        node: impl Into<NodeHandle>,
        buffer: &mut Vec<u8>,
        image_planes: impl AsRef<str>,
        format: impl AsRef<str>,
    ) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::material::extract_image_to_memory(self, node.into(), buffer, image_planes, format)
    }

    // TODO: consider removing this in favour of ['Material'] helper struct which has this API
    pub fn get_image_info(&self, node: impl Into<NodeHandle>) -> Result<ImageInfo> {
        debug_assert!(self.is_valid());
        crate::ffi::get_image_info(self, node.into()).map(ImageInfo)
    }

    /// Render a COP node to a memory buffer
    // TODO: consider removing this in favour of ['Material'] helper struct which has this API
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
                .map(|inner| ImageFileFormat(inner, self.into()))
                .collect()
        })
    }

    pub fn get_active_cache_names(&self) -> Result<StringArray> {
        debug_assert!(self.is_valid());
        crate::ffi::get_active_cache_names(self)
    }

    pub fn get_cache_property_value(
        &self,
        cache_name: &str,
        property: CacheProperty,
    ) -> Result<i32> {
        let cache_name = CString::new(cache_name)?;
        crate::ffi::get_cache_property(self, &cache_name, property)
    }

    pub fn set_cache_property_value(
        &self,
        cache_name: &str,
        property: CacheProperty,
        value: i32,
    ) -> Result<()> {
        let cache_name = CString::new(cache_name)?;
        crate::ffi::set_cache_property(self, &cache_name, property, value)
    }

    pub fn python_thread_interpreter_lock(&self, lock: bool) -> Result<()> {
        debug_assert!(self.is_valid());
        crate::ffi::python_thread_interpreter_lock(self, lock)
    }
    pub fn get_compositor_options(&self) -> Result<CompositorOptions> {
        crate::ffi::get_compositor_options(self).map(CompositorOptions)
    }

    pub fn set_compositor_options(&self, options: &CompositorOptions) -> Result<()> {
        crate::ffi::set_compositor_options(self, &options.0)
    }

    pub fn get_preset_names(&self, bytes: &[u8]) -> Result<Vec<String>> {
        debug_assert!(self.is_valid());
        let mut handles = vec![];
        for handle in crate::ffi::get_preset_names(self, bytes)? {
            let v = crate::stringhandle::get_string(handle, self)?;
            handles.push(v);
        }
        Ok(handles)
    }

    pub fn start_performance_monitor_profile(&self, title: &str) -> Result<i32> {
        let title = CString::new(title)?;
        crate::ffi::start_performance_monitor_profile(self, &title)
    }

    pub fn stop_performance_monitor_profile(
        &self,
        profile_id: i32,
        output_file: &str,
    ) -> Result<()> {
        let output_file = CString::new(output_file)?;
        crate::ffi::stop_performance_monitor_profile(self, profile_id, &output_file)
    }

    #[cfg(feature = "async-cooking")]
    pub fn get_job_status(&self, job_id: i32) -> Result<JobStatus> {
        crate::ffi::get_job_status(self, job_id)
    }
}

impl SessionInner {
    fn finalize(&self) -> Result<()> {
        let mut lifecycle = self.lifecycle.lock();
        Self::finalize_locked_with(
            &self.handle,
            &self.options,
            &mut lifecycle,
            &mut HapiLifecycleBackend,
        )
    }

    fn finalize_locked_with<B: LifecycleBackend>(
        handle: &raw::HAPI_Session,
        options: &SessionOptions,
        lifecycle: &mut LifecycleState,
        backend: &mut B,
    ) -> Result<()> {
        if lifecycle.closed {
            return Ok(());
        }
        // Mark closed before entering HAPI so recursive error handling cannot start teardown again.
        lifecycle.closed = true;
        debug!("Closing session pid: {:?}", lifecycle.server.pid());

        let mut errors = Vec::new();
        if backend.is_valid(handle) {
            if options.cleanup
                && backend.is_initialized(handle)
                && let Err(error) = backend.cleanup(handle)
            {
                errors.push(error.to_string());
            }
            if matches!(lifecycle.server, ServerConnection::InProcess)
                && let Err(error) = backend.shutdown(handle)
            {
                errors.push(error.to_string());
            }
            if let Err(error) = backend.close(handle) {
                errors.push(error.to_string());
            }
        }
        if matches!(lifecycle.server, ServerConnection::InProcess) {
            backend.mark_in_process_shutdown();
        }
        if let Err(error) = backend.finish_server(&lifecycle.server, false) {
            errors.push(error.to_string());
        }
        lifecycle_errors(errors, "Session close")
    }
}

impl Drop for SessionInner {
    fn drop(&mut self) {
        let lifecycle = self.lifecycle.get_mut();
        if let Err(error) = Self::finalize_locked_with(
            &self.handle,
            &self.options,
            lifecycle,
            &mut HapiLifecycleBackend,
        ) {
            error!("Session close failed in Drop: {error}");
        }
    }
}

/// Options passed to `HAPI_Initialize` when creating an initialized [`Session`].
///
/// These settings configure the Houdini runtime after a transport connection
/// exists. Server process and transport settings belong in
/// [`crate::server::ServerOptions`].
#[derive(Default, Clone, Debug)]
pub struct SessionOptions {
    /// Default cook options used by the session.
    pub cook_opt: CookOptions,
    /// Whether Houdini cooks on a separate cooking thread.
    pub threaded: bool,
    /// Run `HAPI_Cleanup` before closing the session.
    ///
    /// This is disabled by default because cleanup can be expensive and normal
    /// server termination releases its Houdini-side state. Enable it only when
    /// that state requires an orderly teardown before the connection closes.
    pub cleanup: bool,
    /// Platform-separated list of Houdini environment files loaded at initialization.
    pub env_files: Option<CString>,
    /// Platform-separated HDA/OTL search paths.
    pub otl_path: Option<CString>,
    /// Platform-separated generic DSO plugin search paths.
    pub dso_path: Option<CString>,
    /// Platform-separated image DSO plugin search paths.
    pub img_dso_path: Option<CString>,
    /// Platform-separated audio DSO plugin search paths.
    pub aud_dso_path: Option<CString>,
}

impl SessionOptions {
    /// A list of Houdini environment files the Engine will load from.
    ///
    /// # Panics
    /// Panics if the joined path list contains an interior null byte.
    #[must_use]
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

    /// Add search paths for the Engine to find HDAs.
    ///
    /// # Panics
    /// Panics if the joined path list contains an interior null byte.
    #[must_use]
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
    ///
    /// # Panics
    /// Panics if the joined path list contains an interior null byte.
    #[must_use]
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
    ///
    /// # Panics
    /// Panics if the joined path list contains an interior null byte.
    #[must_use]
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
    ///
    /// # Panics
    /// Panics if the joined path list contains an interior null byte.
    #[must_use]
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

    /// Pass session [`CookOptions`]
    #[must_use]
    pub fn cook_options(mut self, options: CookOptions) -> Self {
        self.cook_opt = options;
        self
    }

    /// Makes the server operate in threaded mode. See the official docs for more info.
    #[must_use]
    pub fn threaded(mut self, threaded: bool) -> Self {
        self.threaded = threaded;
        self
    }

    /// Set whether to run the potentially expensive `HAPI_Cleanup` before close.
    #[must_use]
    pub fn cleanup(mut self, cleanup: bool) -> Self {
        self.cleanup = cleanup;
        self
    }
}

/// Create an in-process session.
/// Usefull for quick testing and debugging. Session crash will crash the main process.
/// For production use, use [`new_thrift_session`] instead.
pub fn new_in_process_session(options: Option<SessionOptions>) -> Result<Session> {
    debug!("Creating new in-process session");
    if IN_PROCESS_RUNTIME_SHUT_DOWN.load(Ordering::Acquire) {
        return Err(HapiError::Internal(
            "The in-process HAPI runtime has already been shut down and cannot be restarted"
                .to_owned(),
        ));
    }
    let session_options = options.unwrap_or_default();
    let session_info = SessionInfo::default();
    let handle = crate::ffi::create_inprocess_session(&session_info.0)?;
    let session = UninitializedSession {
        session_handle: Some(handle),
        server_options: None,
        server: ServerConnection::InProcess,
    }
    .initialize(session_options)?;
    Ok(session)
}

/// Start a Thrift server and initialize a session with it.
pub fn new_thrift_session(
    session_options: SessionOptions,
    server_options: ServerOptions,
) -> Result<Session> {
    let uninitialized = match &server_options.thrift_transport {
        crate::server::ThriftTransport::SharedMemory(_) => {
            let pid = crate::server::start_engine_server(&server_options)?;
            let connection = crate::server::connect_to_memory_server(server_options, Some(pid))
                .context("Could not connect to shared memory server");
            match connection {
                Ok(mut session) => {
                    session.mark_owned_server(pid);
                    session
                }
                Err(error) => {
                    let pipe_path = None;
                    if let Err(cleanup_error) =
                        crate::server::finish_owned_server(pid, true, true, pipe_path)
                    {
                        error!("Could not roll back HARS after connect failure: {cleanup_error}");
                    }
                    return Err(error);
                }
            }
        }
        crate::server::ThriftTransport::Pipe(_) => {
            let pid = crate::server::start_engine_server(&server_options)?;
            let pipe_path = match &server_options.thrift_transport {
                crate::server::ThriftTransport::Pipe(transport) => {
                    Some(transport.pipe_path.clone())
                }
                _ => unreachable!(),
            };
            let connection = crate::server::connect_to_pipe_server(server_options, Some(pid))
                .context("Could not connect to pipe server");
            match connection {
                Ok(mut session) => {
                    session.mark_owned_server(pid);
                    session
                }
                Err(error) => {
                    if let Err(cleanup_error) =
                        crate::server::finish_owned_server(pid, true, true, pipe_path.as_deref())
                    {
                        error!("Could not roll back HARS after connect failure: {cleanup_error}");
                    }
                    return Err(error);
                }
            }
        }
        crate::server::ThriftTransport::Socket(_) => {
            let pid = crate::server::start_engine_server(&server_options)?;
            let connection = crate::server::connect_to_socket_server(server_options, Some(pid))
                .context("Could not connect to socket server");
            match connection {
                Ok(mut session) => {
                    session.mark_owned_server(pid);
                    session
                }
                Err(error) => {
                    if let Err(cleanup_error) =
                        crate::server::finish_owned_server(pid, true, true, None)
                    {
                        error!("Could not roll back HARS after connect failure: {cleanup_error}");
                    }
                    return Err(error);
                }
            }
        }
    };
    uninitialized.initialize(session_options)
}

/// Shortcut for creating a simple Thrift session with good defaults.
pub fn simple_session() -> Result<Session> {
    new_thrift_session(
        SessionOptions::default(),
        ServerOptions::shared_memory_with_defaults(),
    )
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use std::collections::HashSet;

    struct FakeBackend {
        valid: bool,
        initialized: bool,
        failures: HashSet<&'static str>,
        events: Vec<&'static str>,
    }

    impl FakeBackend {
        fn new(valid: bool, initialized: bool, failures: &[&'static str]) -> Self {
            Self {
                valid,
                initialized,
                failures: failures.iter().copied().collect(),
                events: Vec::new(),
            }
        }

        fn record(&mut self, event: &'static str) -> Result<()> {
            self.events.push(event);
            if self.failures.contains(event) {
                Err(HapiError::Internal(format!("{event} failed")))
            } else {
                Ok(())
            }
        }
    }

    impl LifecycleBackend for FakeBackend {
        fn is_valid(&mut self, _handle: &raw::HAPI_Session) -> bool {
            self.events.push("valid");
            self.valid
        }

        fn is_initialized(&mut self, _handle: &raw::HAPI_Session) -> bool {
            self.events.push("initialized");
            self.initialized
        }

        fn cleanup(&mut self, _handle: &raw::HAPI_Session) -> Result<()> {
            self.record("cleanup")
        }

        fn shutdown(&mut self, _handle: &raw::HAPI_Session) -> Result<()> {
            self.record("shutdown")
        }

        fn close(&mut self, _handle: &raw::HAPI_Session) -> Result<()> {
            self.record("close")
        }

        fn finish_server(
            &mut self,
            _server: &ServerConnection,
            construction_failed: bool,
        ) -> Result<()> {
            self.record(if construction_failed {
                "finish_rollback"
            } else {
                "finish"
            })
        }

        fn mark_in_process_shutdown(&mut self) {
            self.events.push("mark_shutdown");
        }
    }

    fn handle() -> raw::HAPI_Session {
        raw::HAPI_Session {
            type_: SessionType::Thrift,
            id: 7,
        }
    }

    fn borrowed_state() -> LifecycleState {
        LifecycleState {
            closed: false,
            server: ServerConnection::Borrowed { reported_pid: None },
        }
    }

    #[test]
    fn normal_close_skips_opt_in_cleanup_and_is_idempotent() {
        let mut backend = FakeBackend::new(true, true, &[]);
        let mut state = borrowed_state();

        SessionInner::finalize_locked_with(
            &handle(),
            &SessionOptions::default(),
            &mut state,
            &mut backend,
        )
        .unwrap();
        assert!(state.closed);
        assert_eq!(backend.events, ["valid", "close", "finish"]);

        SessionInner::finalize_locked_with(
            &handle(),
            &SessionOptions::default(),
            &mut state,
            &mut backend,
        )
        .unwrap();
        assert_eq!(backend.events, ["valid", "close", "finish"]);
    }

    #[test]
    fn cleanup_close_and_process_errors_are_aggregated_without_short_circuiting() {
        let mut backend = FakeBackend::new(true, true, &["cleanup", "close", "finish"]);
        let mut state = borrowed_state();
        let options = SessionOptions::default().cleanup(true);

        let error =
            SessionInner::finalize_locked_with(&handle(), &options, &mut state, &mut backend)
                .unwrap_err()
                .to_string();

        assert_eq!(
            backend.events,
            ["valid", "initialized", "cleanup", "close", "finish"]
        );
        assert!(error.contains("cleanup failed"));
        assert!(error.contains("close failed"));
        assert!(error.contains("finish failed"));
        assert!(state.closed);
    }

    #[test]
    fn invalid_session_skips_hapi_teardown_but_still_finishes_owned_server() {
        let mut backend = FakeBackend::new(false, false, &[]);
        let mut state = borrowed_state();

        SessionInner::finalize_locked_with(
            &handle(),
            &SessionOptions::default().cleanup(true),
            &mut state,
            &mut backend,
        )
        .unwrap();

        assert_eq!(backend.events, ["valid", "finish"]);
    }

    #[test]
    fn in_process_close_shuts_down_and_marks_global_runtime() {
        let mut backend = FakeBackend::new(true, true, &[]);
        let mut state = LifecycleState {
            closed: false,
            server: ServerConnection::InProcess,
        };

        SessionInner::finalize_locked_with(
            &handle(),
            &SessionOptions::default(),
            &mut state,
            &mut backend,
        )
        .unwrap();

        assert_eq!(
            backend.events,
            ["valid", "shutdown", "close", "mark_shutdown", "finish"]
        );
    }

    #[test]
    fn in_process_shutdown_failure_does_not_skip_close_or_runtime_reset() {
        let mut backend = FakeBackend::new(true, true, &["shutdown"]);
        let mut state = LifecycleState {
            closed: false,
            server: ServerConnection::InProcess,
        };

        let error = SessionInner::finalize_locked_with(
            &handle(),
            &SessionOptions::default(),
            &mut state,
            &mut backend,
        )
        .unwrap_err()
        .to_string();

        assert_eq!(
            backend.events,
            ["valid", "shutdown", "close", "mark_shutdown", "finish"]
        );
        assert!(error.contains("shutdown failed"));
        assert!(state.closed);
    }

    #[test]
    fn initialized_connection_rollback_cleans_closes_and_finishes_server() {
        let mut backend = FakeBackend::new(true, true, &[]);

        rollback_uninitialized_with(
            Some(&handle()),
            &ServerConnection::Borrowed { reported_pid: None },
            &mut backend,
        )
        .unwrap();

        assert_eq!(
            backend.events,
            [
                "valid",
                "initialized",
                "cleanup",
                "close",
                "finish_rollback"
            ]
        );
    }

    #[test]
    fn rollback_without_a_handle_only_finishes_the_server() {
        let mut backend = FakeBackend::new(true, true, &[]);

        rollback_uninitialized_with(
            None,
            &ServerConnection::Borrowed { reported_pid: None },
            &mut backend,
        )
        .unwrap();

        assert_eq!(backend.events, ["finish_rollback"]);
    }

    #[test]
    fn valid_but_uninitialized_rollback_skips_cleanup() {
        let mut backend = FakeBackend::new(true, false, &[]);

        rollback_uninitialized_with(
            Some(&handle()),
            &ServerConnection::Borrowed { reported_pid: None },
            &mut backend,
        )
        .unwrap();

        assert_eq!(
            backend.events,
            ["valid", "initialized", "close", "finish_rollback"]
        );
    }

    #[test]
    fn invalid_in_process_rollback_still_resets_global_runtime() {
        let mut backend = FakeBackend::new(false, false, &[]);

        rollback_uninitialized_with(Some(&handle()), &ServerConnection::InProcess, &mut backend)
            .unwrap();

        assert_eq!(
            backend.events,
            ["valid", "mark_shutdown", "finish_rollback"]
        );
    }

    #[test]
    fn cleanup_is_skipped_when_session_was_not_initialized() {
        let mut backend = FakeBackend::new(true, false, &[]);
        let mut state = borrowed_state();

        SessionInner::finalize_locked_with(
            &handle(),
            &SessionOptions::default().cleanup(true),
            &mut state,
            &mut backend,
        )
        .unwrap();

        assert_eq!(backend.events, ["valid", "initialized", "close", "finish"]);
    }

    #[test]
    fn rollback_attempts_every_stage_and_aggregates_failures() {
        let mut backend = FakeBackend::new(
            true,
            true,
            &["cleanup", "shutdown", "close", "finish_rollback"],
        );

        let error = rollback_uninitialized_with(
            Some(&handle()),
            &ServerConnection::InProcess,
            &mut backend,
        )
        .unwrap_err()
        .to_string();

        assert_eq!(
            backend.events,
            [
                "valid",
                "initialized",
                "cleanup",
                "shutdown",
                "close",
                "mark_shutdown",
                "finish_rollback"
            ]
        );
        for failure in ["cleanup", "shutdown", "close", "finish_rollback"] {
            assert!(error.contains(&format!("{failure} failed")));
        }
    }
}
