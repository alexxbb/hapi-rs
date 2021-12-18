#[rustfmt::skip]
use std::{
    ffi::CString,
    sync::Arc,
};
use log::{debug, error, warn};

pub use crate::ffi::PartInfo;
pub use crate::{
    asset::AssetLibrary,
    errors::*,
    ffi::raw::{HapiResult, State, StatusType, StatusVerbosity, HAPI_Session},
    ffi::{CookOptions, SessionSyncInfo, TimelineOptions, Viewport},
    node::{HoudiniNode, NodeHandle},
    stringhandle::StringArray,
};

use parking_lot::ReentrantMutex;


impl std::cmp::PartialEq for crate::ffi::raw::HAPI_Session {
    fn eq(&self, other: &Self) -> bool {
        self.type_ == other.type_ && self.id == other.id
    }
}

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
        session.get_string(crate::ffi::get_server_env_str(session, &key)?)
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

#[derive(Debug, Clone)]
pub enum CookResult {
    Succeeded,
    Warnings,
    Errored(String),
}

#[derive(Debug, Clone)]
pub struct Session {
    pub(crate) handle: Arc<(HAPI_Session, ReentrantMutex<()>)>,
    cleanup: bool,
    pub threaded: bool,
}

impl Session {
    #[inline]
    pub(crate) fn ptr(&self) -> *const HAPI_Session {
        &(self.handle.0) as *const _
    }
    #[inline]
    pub fn uid(&self) -> i64 {
        self.handle.0.id
    }

    pub fn set_server_var<T: EnvVariable + ?Sized>(
        &self,
        key: &str,
        value: &T::Type,
    ) -> Result<()> {
        T::set_value(self, key, value)
    }

    pub fn get_server_var<T: EnvVariable + ?Sized>(
        &self,
        key: &str,
    ) -> Result<<T::Type as ToOwned>::Owned> {
        T::get_value(self, key)
    }

    pub fn get_server_variables(&self) -> Result<StringArray> {
        let count = crate::ffi::get_server_env_var_count(self)?;
        let handles = crate::ffi::get_server_env_var_list(self, count)?;
        crate::stringhandle::get_string_array(&handles, self)
    }

    pub fn initialize(&mut self, opts: &SessionOptions) -> Result<()> {
        debug!("Initializing session");
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

    pub fn cleanup(&self) -> Result<()> {
        debug!("Cleaning session");
        crate::ffi::cleanup_session(self)
    }

    pub fn is_initialized(&self) -> bool {
        crate::ffi::is_session_initialized(self)
    }

    pub fn create_input_node(&self, name: &str) -> Result<HoudiniNode> {
        let name = CString::new(name)?;
        let id = crate::ffi::create_input_node(self, &name)?;
        HoudiniNode::new(self.clone(), NodeHandle(id, ()), None)
    }
    pub fn create_node<'a>(
        &self,
        name: &str,
        label: impl Into<Option<&'a str>>,
        parent: Option<NodeHandle>,
    ) -> Result<HoudiniNode> {
        HoudiniNode::create(name, label.into(), parent, self.clone(), false)
    }

    pub fn create_node_blocking(
        &self,
        name: &str,
        label: Option<&str>,
        parent: Option<NodeHandle>,
    ) -> Result<HoudiniNode> {
        HoudiniNode::create_blocking(name, label, parent, self.clone(), false)
    }

    pub fn save_hip(&self, name: &str, lock_nodes: bool) -> Result<()> {
        debug!("Saving hip file: {}", name);
        let name = CString::new(name)?;
        crate::ffi::save_hip(self, &name, lock_nodes)
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

    pub fn get_cook_result_string(&self, verbosity: StatusVerbosity) -> Result<String> {
        self.get_status_string(StatusType::CookResult, verbosity)
    }

    pub fn cooking_total_count(&self) -> Result<i32> {
        crate::ffi::get_cooking_total_count(self)
    }

    pub fn cooking_current_count(&self) -> Result<i32> {
        crate::ffi::get_cooking_current_count(self)
    }

    /// In threaded mode wait for Session finishes cooking. In single thread mode, immediately return
    pub fn cook(&self) -> Result<CookResult> {
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

    pub fn get_connection_error(&self, clear: bool) -> Result<String> {
        crate::ffi::get_connection_error(clear)
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

    pub fn get_viewport(&self) -> Result<Viewport> {
        crate::ffi::get_viewport(self).map(|inner| Viewport { inner })
    }

    pub fn set_viewport(&self, viewport: &Viewport) -> Result<()> {
        crate::ffi::set_viewport(self, viewport)
    }

    pub fn set_sync(&self, enable: bool) -> Result<()> {
        crate::ffi::set_session_sync(self, enable)
    }
    pub fn get_sync_info(&self) -> Result<SessionSyncInfo> {
        crate::ffi::get_session_sync_info(self).map(|inner| SessionSyncInfo { inner })
    }

    pub fn set_sync_info(&self, info: &SessionSyncInfo) -> Result<()> {
        crate::ffi::set_session_sync_info(self, &info.inner)
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
                if let Err(e) = crate::ffi::close_session(self) {
                    error!("Closing session failed in Drop: {}", e);
                }
            }
        }
    }
}

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

pub struct SessionOptions {
    pub cook_opt: CookOptions,
    pub threaded: bool,
    pub cleanup: bool,
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

pub fn start_engine_pipe_server(path: &str, auto_close: bool, timeout: f32) -> Result<u32> {
    debug!("Starting named pipe server: {}", path);
    let path = CString::new(path)?;
    let opts = crate::ffi::raw::HAPI_ThriftServerOptions {
        autoClose: auto_close as i8,
        timeoutMs: timeout,
    };
    crate::ffi::start_thrift_pipe_server(&path, &opts)
}
pub fn start_engine_socket_server(port: u16, auto_close: bool, timeout: i32) -> Result<u32> {
    debug!("Starting socket server on port: {}", port);
    let opts = crate::ffi::raw::HAPI_ThriftServerOptions {
        autoClose: auto_close as i8,
        timeoutMs: timeout as f32,
    };
    crate::ffi::start_thrift_socket_server(port as i32, &opts)
}

pub fn simple_session(options: Option<&SessionOptions>) -> Result<Session> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;
    let mut hash = DefaultHasher::new();
    SystemTime::now().hash(&mut hash);
    std::thread::current().id().hash(&mut hash);
    let file = std::env::temp_dir().join(format!("hars-session-{}", hash.finish()));
    let file = file.to_string_lossy();
    start_engine_pipe_server(&file, true, 4000.0)?;
    let mut session = connect_to_pipe(&file)?;
    session.initialize(options.unwrap_or(&SessionOptions::default()))?;
    Ok(session)
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::session::*;
    use once_cell::sync::Lazy;
    use std::collections::HashMap;

    static SESSION: Lazy<Session> = Lazy::new(|| {
        env_logger::init();
        simple_session(None).expect("Could not create test session")
    });

    pub(crate) fn with_session(func: impl FnOnce(&Lazy<Session>)) {
        func(&SESSION);
    }

    pub(crate) static OTLS: Lazy<HashMap<&str, String>> = Lazy::new(|| {
        let mut map = HashMap::new();
        let root = format!(
            "{}/otls",
            std::env::current_dir()
                .unwrap()
                .parent()
                .unwrap()
                .to_string_lossy()
        );
        map.insert("geometry", format!("{}/hapi_geo.hda", root));
        map.insert("parameters", format!("{}/hapi_parms.hda", root));
        map.insert("spaceship", format!("{}/sesi/SideFX_spaceship.otl", root));
        map
    });

    #[test]
    fn init_and_teardown() {
        let mut opt = super::SessionOptions::default();
        opt.set_dso_search_paths(["/path/one", "/path/two"]);
        opt.set_otl_search_paths(["/path/thee", "/path/four"]);
        let mut ses = super::simple_session(Some(&opt)).unwrap();
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
            let opt = crate::TimelineOptions::default().with_end_time(5.5);
            assert!(session.set_timeline_options(opt.clone()).is_ok());
            let opt2 = session.get_timeline_options().expect("timeline_options");
            assert!(opt.end_time().eq(&opt2.end_time()));
            session.set_time(4.12).expect("set_time");
            session.cook().unwrap();
            assert_eq!(session.get_time().expect("get_time"), 4.12);
        });
    }

    #[test]
    fn server_variables() {
        // Starting new separate session because getting/setting env variables from multiple
        // clients ( threads ) break the server
        let session = super::simple_session(None).expect("Could not start session");
        session.set_server_var::<str>("FOO", "foo_string").unwrap();
        assert_eq!(session.get_server_var::<str>("FOO").unwrap(), "foo_string");
        session.set_server_var::<i32>("BAR", &123).unwrap();
        assert_eq!(session.get_server_var::<i32>("BAR").unwrap(), 123);
        assert!(!session.get_server_variables().unwrap().is_empty());
    }

    #[test]
    fn create_node_blocking() {
        with_session(|session| {
            session
                .create_node("Object/geo", Some("some_name"), None)
                .unwrap();
        });
    }

    #[test]
    fn load_asset_library() {
        with_session(|session| {
            assert!(session.load_asset_file(&OTLS["spaceship"]).is_ok());
        });
    }

    #[test]
    fn load_asset() {
        with_session(|session| {
            let otl = OTLS.get("parameters").unwrap();
            let lib = session.load_asset_file(otl);
            assert!(lib.is_ok(), "load_asset_file");
        });
    }

    #[test]
    fn create_node_async() {
        use crate::ffi::raw::{NodeFlags, NodeType};
        let mut opt = super::SessionOptions::default();
        opt.threaded = true;
        let session = super::simple_session(Some(&opt)).unwrap();
        let lib = session.load_asset_file(&OTLS["spaceship"]).unwrap();
        let node = lib.try_create_first().unwrap();
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
