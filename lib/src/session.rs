//! Session is responsible for communicating with HAPI
//!
//! The Engine [promises](https://www.sidefx.com/docs/hengine/_h_a_p_i__sessions.html#HAPI_Sessions_Multithreading)
//! to be thread-safe when accessing a single `Session` from multiple threads.
//! `hapi-rs` relies on this promise and the [Session] struct holds only an `Arc` pointer to the session,
//! and *does not* protect the session with Mutex, although there is a [ReentrantMutex]
//! private member which is used internally in a few cases where API calls must be sequential.
//!
//! When the last instance of the `Session` is about to get dropped, it'll be cleaned up
//! (if [SessionOptions::cleanup] was set) and automatically closed.
//!
//! The Engine process (pipe or socket) can be auto-terminated as well if told so when starting the server:
//! See [`crate::server::start_engine_pipe_server`] and [`crate::server::start_engine_socket_server`]
//!
//! Helper constructors terminate the server by default. This is useful for quick one-off jobs.
//!
use log::{debug, error};
use parking_lot::ReentrantMutex;
use std::fmt::Debug;
use std::path::PathBuf;
use std::{ffi::CString, path::Path, sync::Arc};

pub use crate::{
    asset::AssetLibrary,
    errors::*,
    ffi::{
        CompositorOptions, CookOptions, ImageFileFormat, SessionInfo, SessionSyncInfo,
        ThriftServerOptions, TimelineOptions, Viewport, enums::*,
    },
    node::{HoudiniNode, ManagerNode, ManagerType, NodeHandle, NodeType, Transform},
    parameter::Parameter,
    server::ServerOptions,
    stringhandle::StringArray,
};

// A result of HAPI_GetStatus with HAPI_STATUS_COOK_STATE
pub type SessionState = State;

use crate::ffi::ImageInfo;
use crate::stringhandle::StringHandle;
use crate::{ffi::raw, utils};

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
    Succeeded,
    /// Some nodes cooked with errors
    CookErrors(String),
    /// One or more nodes could not cook - should abort cooking
    FatalErrors(String),
}

impl CookResult {
    /// Convenient method for cook result message if any
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Succeeded => None,
            Self::CookErrors(msg) => Some(msg.as_str()),
            Self::FatalErrors(msg) => Some(msg.as_str()),
        }
    }
}

/// By which means the session communicates with the server.
#[derive(Debug)]
pub(crate) struct SessionInner {
    pub(crate) handle: raw::HAPI_Session,
    pub(crate) options: SessionOptions,
    // Server options are only available for Thrift servers.
    pub(crate) server_options: Option<ServerOptions>,
    pub(crate) lock: ReentrantMutex<()>,
    pub(crate) server_pid: Option<u32>,
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

pub struct UninitializedSession {
    pub(crate) session_handle: raw::HAPI_Session,
    pub(crate) server_options: ServerOptions,
    pub(crate) server_pid: Option<u32>,
}

impl UninitializedSession {
    pub fn initialize(self, session_options: SessionOptions) -> Result<Session> {
        debug!("Initializing session");
        crate::ffi::initialize_session(self.session_handle, &session_options)
            .map(|_| Session {
                inner: Arc::new(SessionInner {
                    handle: self.session_handle,
                    options: session_options,
                    lock: ReentrantMutex::new(()),
                    server_options: Some(self.server_options),
                    server_pid: self.server_pid,
                }),
            })
            .with_context(|| "Calling initialize_session")
    }
}

impl Session {
    /// Return [`SessionType`] current session is initialized with.
    pub fn session_type(&self) -> SessionType {
        self.inner.handle.type_
    }

    /// Return enum with extra connection data such as pipe file or socket.

    pub fn server_pid(&self) -> Option<u32> {
        self.inner.server_pid
    }

    #[inline(always)]
    pub(crate) fn ptr(&self) -> *const raw::HAPI_Session {
        &(self.inner.handle) as *const _
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
        T::get_value(self, key)
    }

    /// Retrieve all server variables
    pub fn get_server_variables(&self) -> Result<StringArray> {
        debug_assert!(self.is_valid());
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

    /// Consumes and cleanups up the session. Session becomes invalid after this call
    pub fn cleanup(self) -> Result<()> {
        debug!("Cleaning session");
        debug_assert!(self.is_valid());
        crate::ffi::cleanup_session(&self)
    }

    /// Create an input geometry node which can accept modifications
    pub fn create_input_node(
        &self,
        name: &str,
        parent: Option<NodeHandle>,
    ) -> Result<crate::geometry::Geometry> {
        debug!("Creating input node: {}", name);
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        let id = crate::ffi::create_input_node(self, &name, parent)?;
        let node = HoudiniNode::new(self.clone(), NodeHandle(id), None)?;
        let info = crate::geometry::GeoInfo::from_node(&node)?;
        Ok(crate::geometry::Geometry { node, info })
    }

    /// Create an input geometry node with [`PartType`] set to `Curve`
    pub fn create_input_curve_node(
        &self,
        name: &str,
        parent: Option<NodeHandle>,
    ) -> Result<crate::geometry::Geometry> {
        debug!("Creating input curve node: {}", name);
        debug_assert!(self.is_valid());
        let name = CString::new(name)?;
        let id = crate::ffi::create_input_curve_node(self, &name, parent)?;
        let node = HoudiniNode::new(self.clone(), NodeHandle(id), None)?;
        let info = crate::geometry::GeoInfo::from_node(&node)?;
        Ok(crate::geometry::Geometry { node, info })
    }

    /// Create a node. `name` must start with a network category, e.g, "Object/geo", "Sop/box",
    /// in operator namespace was used, the full name may look like this: namespace::Object/mynode
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
        debug!("Creating node instance: {}", name);
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
        crate::ffi::delete_node(node.into(), self)
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
            debug!("Node {} not found", path);
            return Ok(None);
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
        debug!("Saving hip file: {:?}", path.as_ref());
        debug_assert!(self.is_valid());
        let path = utils::path_to_cstring(path)?;
        crate::ffi::save_hip(self, &path, lock_nodes)
    }

    /// Load a hip file into current session
    pub fn load_hip(&self, path: impl AsRef<Path>, cook: bool) -> Result<()> {
        debug!("Loading hip file: {:?}", path.as_ref());
        debug_assert!(self.is_valid());
        let path = utils::path_to_cstring(path)?;
        crate::ffi::load_hip(self, &path, cook)
    }

    /// Merge a hip file into current session
    pub fn merge_hip(&self, name: &str, cook: bool) -> Result<i32> {
        debug!("Merging hip file: {}", name);
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
                crate::ffi::get_asset_library_file_path(self, library_id).map(|lib_file| {
                    AssetLibrary {
                        lib_id: library_id,
                        session: self.clone(),
                        file: Some(PathBuf::from(lib_file)),
                    }
                })
            })
            .collect::<Result<Vec<_>>>()
    }

    /// Interrupt session cooking
    pub fn interrupt(&self) -> Result<()> {
        debug_assert!(self.is_valid());
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
    #[inline(always)]
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
    pub fn set_timeline_options(&self, options: TimelineOptions) -> Result<()> {
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

    pub fn extract_image_to_file(
        &self,
        node: impl Into<NodeHandle>,
        image_planes: &str,
        path: impl AsRef<Path>,
    ) -> Result<String> {
        crate::material::extract_image_to_file(self, node.into(), image_planes, path)
    }

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

    pub fn get_image_info(&self, node: impl Into<NodeHandle>) -> Result<ImageInfo> {
        debug_assert!(self.is_valid());
        crate::ffi::get_image_info(self, node.into()).map(ImageInfo)
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

    pub fn get_job_status(&self, job_id: i32) -> Result<JobStatus> {
        crate::ffi::get_job_status(self, job_id)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            debug!("Dropping session pid: {:?}", self.server_pid());
            if self.is_valid() {
                if self.inner.options.cleanup
                    && let Err(e) = crate::ffi::cleanup_session(self)
                {
                    error!("Session cleanup failed in Drop: {}", e);
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
                debug!("Session was invalid in Drop!");
                if let Some(server_options) = &self.inner.server_options {
                    if let crate::server::ThriftTransport::Pipe(transport) =
                        &server_options.thrift_transport
                    {
                        let _ = std::fs::remove_file(&transport.pipe_path);
                    }
                }
            }
        }
    }
}

/// Session options passed to session create functions like [`crate::server::connect_to_pipe_server`]
#[derive(Clone, Debug)]
pub struct SessionOptions {
    /// Session cook options
    pub cook_opt: CookOptions,
    /// Create a Threaded server connection
    pub threaded: bool,
    /// Cleanup session upon close
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
            threaded: false,
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
    /// A list of Houdini environment files the Engine will load from.
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
}

/// Create an in-process session.
/// Usefull for quick testing and debugging. Session crash will crash the main process.
/// For production use, use [`new_thrift_session`] instead.
pub fn new_in_process_session(options: Option<SessionOptions>) -> Result<Session> {
    debug!("Creating new in-process session");
    let session_options = options.unwrap_or_default();
    let session_info = SessionInfo::default();
    let handle = crate::ffi::create_inprocess_session(&session_info.0)?;
    Ok(Session {
        inner: Arc::new(SessionInner {
            handle,
            options: session_options,
            lock: ReentrantMutex::new(()),
            server_options: None,
            server_pid: Some(std::process::id()),
        }),
    })
}

/// Start a Thrift server and initialize a session with it.
pub fn new_thrift_session(
    session_options: SessionOptions,
    server_options: ServerOptions,
) -> Result<Session> {
    match server_options.thrift_transport {
        crate::server::ThriftTransport::SharedMemory(_) => {
            let pid = crate::server::start_engine_server(&server_options)?;
            crate::server::connect_to_memory_server(server_options, Some(pid))
                .context("Could not connect to shared memory server")?
                .initialize(session_options)
        }
        crate::server::ThriftTransport::Pipe(_) => {
            let pid = crate::server::start_engine_server(&server_options)?;
            crate::server::connect_to_pipe_server(server_options, Some(pid))
                .context("Could not connect to pipe server")?
                .initialize(session_options)
        }
        crate::server::ThriftTransport::Socket(_) => {
            let pid = crate::server::start_engine_server(&server_options)?;
            crate::server::connect_to_socket_server(server_options, Some(pid))
                .context("Could not connect to socket server")?
                .initialize(session_options)
                .context("Could not connect to socket server")
        }
    }
}
