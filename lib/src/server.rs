use std::{
    collections::HashMap,
    ffi::{CString, OsStr, OsString},
    net::SocketAddrV4,
    num::NonZeroU64,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use log::{debug, error, warn};
use temp_env;

use crate::{
    errors::{ErrorContext, HapiError, Result},
    ffi::{self, ThriftServerOptions, enums::StatusVerbosity},
    session::UninitializedSession,
    utils,
};

pub use crate::ffi::raw::ThriftSharedMemoryBufferType;

/// Controls which Houdini product licenses HARS may check out.
///
/// The preference is passed to the server through `HOUDINI_PLUGIN_LIC_OPT`
/// before HARS starts. It has no effect when merely attaching to an already
/// running server.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LicensePreference {
    /// Allow Houdini Engine, Houdini Core, or Houdini FX licenses.
    AnyAvailable,
    /// Restrict checkout to Houdini Engine licenses.
    HoudiniEngineOnly,
    /// Allow Houdini Engine or Houdini Core, but not Houdini FX.
    HoudiniEngineAndCore,
}

impl std::fmt::Display for LicensePreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LicensePreference::AnyAvailable => {
                    "--check-licenses=Houdini-Engine,Houdini-Escape,Houdini-Fx"
                }
                LicensePreference::HoudiniEngineOnly => {
                    "--check-licenses=Houdini-Engine --skip-licenses=Houdini-Escape,Houdini-Fx"
                }
                LicensePreference::HoudiniEngineAndCore => {
                    "--check-licenses=Houdini-Engine,Houdini-Escape --skip-licenses=Houdini-Fx"
                }
            }
        )
    }
}

/// Configuration for a local Thrift shared-memory transport.
///
/// The buffer type and size are supplied to both HARS and the client session;
/// HAPI requires the two sides to match.
#[derive(Clone, Debug)]
pub struct ThriftSharedMemoryTransport {
    /// Name of the platform shared-memory object used by HARS and HARC.
    pub memory_name: String,
    /// Fixed-length or ring-buffer transport mode.
    pub buffer_type: ThriftSharedMemoryBufferType,
    /// Shared-memory buffer size in megabytes.
    pub buffer_size: i64,
}

/// Configuration for a TCP Thrift transport.
#[derive(Clone, Debug)]
pub struct ThriftSocketTransport {
    /// IPv4 address and port on which HARS listens or to which HARC connects.
    pub address: SocketAddrV4,
}

/// Configuration for a named-pipe Thrift transport.
///
/// On Unix-like systems HAPI implements this transport as a Unix domain
/// socket; on Windows it is a native named pipe.
#[derive(Clone, Debug)]
pub struct ThriftPipeTransport {
    /// Pipe name or Unix domain socket path shared by HARS and HARC.
    pub pipe_path: PathBuf,
}

/// Transport used to communicate with a Thrift HARS server.
#[derive(Clone, Debug)]
pub enum ThriftTransport {
    /// Local shared-memory transport.
    SharedMemory(ThriftSharedMemoryTransport),
    /// Named pipe or Unix domain socket transport.
    Pipe(ThriftPipeTransport),
    /// TCP socket transport.
    Socket(ThriftSocketTransport),
}

/// Builder for [`ThriftSharedMemoryTransport`].
///
/// Defaults to a random memory name, HAPI's fixed-length buffer mode, and the
/// official HAPI default size of 100 MB.
pub struct ThriftSharedMemoryTransportBuilder {
    memory_name: String,
    buffer_type: ThriftSharedMemoryBufferType,
    buffer_size: i64,
}

impl Default for ThriftSharedMemoryTransportBuilder {
    fn default() -> Self {
        Self {
            memory_name: format!("shared-memory-{}", utils::random_string(16)),
            buffer_type: ThriftSharedMemoryBufferType::Buffer,
            // Match HAPI_ThriftServerOptions_Create and HAPI_SessionInfo_Create.
            buffer_size: 100, // MB
        }
    }
}

impl ThriftSharedMemoryTransportBuilder {
    #[must_use]
    pub fn with_memory_name(mut self, name: impl Into<String>) -> Self {
        self.memory_name = name.into();
        self
    }
    #[must_use]
    pub fn with_buffer_type(mut self, buffer_type: ThriftSharedMemoryBufferType) -> Self {
        self.buffer_type = buffer_type;
        self
    }
    #[must_use]
    pub fn with_buffer_size(mut self, buffer_size: NonZeroU64) -> Self {
        self.buffer_size = if let Ok(size) = buffer_size.get().try_into() {
            size
        } else {
            // When u64 can't fit into i64, use the HAPI default of 100 MB.
            warn!("ThriftSharedMemoryTransport buffer size is too large, using default of 100");
            100
        };
        self
    }
    #[must_use]
    pub fn build(self) -> ThriftSharedMemoryTransport {
        ThriftSharedMemoryTransport {
            memory_name: self.memory_name,
            buffer_type: self.buffer_type,
            buffer_size: self.buffer_size,
        }
    }
}

/// Configuration used to start or connect to a Thrift HARS server.
///
/// Server-start settings such as [`Self::auto_close`], [`Self::verbosity`],
/// [`Self::log_file`], and [`Self::server_ready_timeout`] only affect a server
/// started with [`start_engine_server`] or [`crate::session::new_thrift_session`].
/// They cannot reconfigure an existing server used by the `connect_to_*`
/// helpers.
///
/// The default selects shared memory with a random name, enables HARS
/// auto-close, and retries client connection attempts for ten seconds.
#[derive(Clone, Debug)]
pub struct ServerOptions {
    /// Thrift transport and endpoint configuration.
    pub thrift_transport: ThriftTransport,
    /// Ask a newly started HARS process to exit after its last client closes.
    pub auto_close: bool,
    /// Maximum HARS log verbosity.
    pub verbosity: StatusVerbosity,
    /// Optional file to which a newly started HARS writes its log.
    pub log_file: Option<CString>,
    /// Process environment entries temporarily applied while starting HARS.
    pub env_variables: Option<HashMap<OsString, OsString>>,
    /// Optional Houdini license checkout preference.
    pub license_preference: Option<LicensePreference>,
    /// Number of HARC subconnections requested for the session.
    pub connection_count: i32,
    /// Optional HARS readiness timeout, in milliseconds.
    ///
    /// When absent, the default supplied by HAPI is retained.
    pub server_ready_timeout: Option<u32>,
    pub(crate) connection_retry_interval: Option<Duration>,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            thrift_transport: ThriftTransport::SharedMemory(
                ThriftSharedMemoryTransportBuilder::default().build(),
            ),
            auto_close: true,
            verbosity: StatusVerbosity::Statusverbosity0,
            log_file: None,
            env_variables: None,
            license_preference: None,
            connection_count: 0,
            server_ready_timeout: None,
            connection_retry_interval: Some(Duration::from_secs(10)),
        }
    }
}

impl ServerOptions {
    /// Create options for a shared-memory transport with a random name.
    #[must_use]
    pub fn shared_memory_with_defaults() -> Self {
        Self::default().with_thrift_transport(ThriftTransport::SharedMemory(
            ThriftSharedMemoryTransportBuilder::default().build(),
        ))
    }

    /// Create options for a named pipe transport.
    #[must_use]
    pub fn pipe_with_defaults() -> Self {
        Self::default().with_thrift_transport(ThriftTransport::Pipe(ThriftPipeTransport {
            pipe_path: PathBuf::from(format!("hapi-pipe-{}", utils::random_string(16))),
        }))
    }

    /// Create options for a socket transport.
    #[must_use]
    pub fn socket_with_defaults(address: SocketAddrV4) -> Self {
        Self::default()
            .with_thrift_transport(ThriftTransport::Socket(ThriftSocketTransport { address }))
    }

    #[must_use]
    /// Replace the configured Thrift transport.
    pub fn with_thrift_transport(mut self, transport: ThriftTransport) -> Self {
        self.thrift_transport = transport;
        self
    }

    /// Set the total retry timeout used while establishing a Thrift session.
    ///
    /// `None` retries indefinitely. This is separate from
    /// [`Self::with_server_ready_timeout`], which controls server startup.
    #[must_use]
    pub fn with_connection_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.connection_retry_interval = timeout;
        self
    }

    /// Set the license preference for the server.
    /// For more information, see <https://www.sidefx.com/docs/houdini//licensing/system.html>
    /// Default is No preference, the server decides which license to check out.
    #[must_use]
    pub fn with_license_preference(mut self, license_preference: LicensePreference) -> Self {
        self.license_preference.replace(license_preference);

        self.env_variables.get_or_insert_default().insert(
            OsString::from("HOUDINI_PLUGIN_LIC_OPT"),
            OsString::from(license_preference.to_string()),
        );

        self
    }

    /// Set the log file for the server.
    /// BUG: HARS 21.0.685 has a bug where the log file is always created in the working directory
    ///
    /// # Panics
    /// Panics if `file` contains an interior null byte and cannot be converted to a C string.
    #[must_use]
    pub fn with_log_file(mut self, file: impl AsRef<Path>) -> Self {
        self.log_file = Some(utils::path_to_cstring(file).expect("Path to CString failed"));
        self
    }

    /// Set **real** environment variables before the server starts.
    /// Unlike [`crate::session::Session::set_server_var`], where the variables are set in the session after the
    /// server starts.
    #[must_use]
    pub fn with_env_variables<'a, I, K, V>(mut self, variables: I) -> Self
    where
        I: Iterator<Item = &'a (K, V)>,
        K: Into<OsString> + Clone + 'a,
        V: Into<OsString> + Clone + 'a,
    {
        self.env_variables
            .get_or_insert_default()
            .extend(variables.map(|(k, v)| (k.clone().into(), v.clone().into())));
        self
    }

    /// Automatically close the server when the last connection drops.
    #[must_use]
    pub fn with_auto_close(mut self, auto_close: bool) -> Self {
        self.auto_close = auto_close;
        self
    }

    /// Set the verbosity level for the server.
    #[must_use]
    pub fn with_verbosity(mut self, verbosity: StatusVerbosity) -> Self {
        self.verbosity = verbosity;
        self
    }

    #[must_use]
    #[cfg(feature = "async-cooking")]
    /// Set the number of HARC subconnections used by asynchronous HAPI calls.
    pub fn with_connection_count(mut self, connection_count: i32) -> Self {
        // BUG: HARS 21.0.* has a bug where the connection count is not respected.
        // If connection_count is > 0, there is a bug in HARS which prevents session creation.
        // However, async attribute access requires a connection count > 0 according to SESI support, otherwise HARS crashes too.
        self.connection_count = connection_count;
        self
    }

    /// Set the timeout for the server to be ready in ms
    /// This is the timeout for the server to initialize and be ready to accept connections.
    #[must_use]
    pub fn with_server_ready_timeout(mut self, timeout: u32) -> Self {
        self.server_ready_timeout.replace(timeout);
        self
    }

    pub(crate) fn session_info(&self) -> crate::ffi::SessionInfo {
        let mut session_info =
            crate::ffi::SessionInfo::default().with_connection_count(self.connection_count);

        if let ThriftTransport::SharedMemory(transport) = &self.thrift_transport {
            session_info.set_shared_memory_buffer_type(transport.buffer_type);
            session_info.set_shared_memory_buffer_size(transport.buffer_size);
        }

        session_info
    }

    pub(crate) fn thrift_options(&self) -> crate::ffi::ThriftServerOptions {
        let mut options = ThriftServerOptions::default()
            .with_auto_close(self.auto_close)
            .with_verbosity(self.verbosity);

        if let ThriftTransport::SharedMemory(transport) = &self.thrift_transport {
            options.set_shared_memory_buffer_type(transport.buffer_type);
            options.set_shared_memory_buffer_size(transport.buffer_size);
        }
        if let Some(timeout) = self.server_ready_timeout {
            #[allow(clippy::cast_precision_loss)]
            options.set_timeout_ms(timeout as f32);
        }

        options
    }
}

fn call_with_temp_environment<R, T, F>(variables: Option<&[(T, T)]>, f: F) -> Result<R>
where
    T: AsRef<OsStr>,
    F: FnOnce() -> Result<R>,
{
    if let Some(env_variables) = variables {
        let env_variables: Vec<(&OsStr, Option<&OsStr>)> = env_variables
            .iter()
            .map(|(k, v)| (k.as_ref(), Some(v.as_ref())))
            .collect::<Vec<_>>();
        temp_env::with_vars(env_variables.as_slice(), f)
    } else {
        f()
    }
}

/// Connect to the Thrift pipe server and return an uninitialized session.
pub fn connect_to_pipe_server(
    server_options: ServerOptions,
    pid: Option<u32>,
) -> Result<UninitializedSession> {
    validate_server_options(&server_options)?;
    let ThriftTransport::Pipe(ThriftPipeTransport { pipe_path }) = &server_options.thrift_transport
    else {
        return Err(HapiError::Internal(
            "ServerOptions is not configured for pipe transport".to_owned(),
        ));
    };
    let pipe_name = utils::path_to_cstring(pipe_path)?;
    debug!("Connecting to pipe server: {:?}", pipe_path.display());
    let handle = try_connect_with_timeout(
        server_options.connection_retry_interval,
        Duration::from_millis(100),
        || ffi::new_thrift_piped_session(&pipe_name, &server_options.session_info().0),
    )?;
    Ok(UninitializedSession {
        session_handle: Some(handle),
        server_options: Some(server_options),
        server: crate::session::ServerConnection::Borrowed { reported_pid: pid },
    })
}

/// Connect to the Thrift shared memory server and return an uninitialized session.
pub fn connect_to_memory_server(
    server_options: ServerOptions,
    pid: Option<u32>,
) -> Result<UninitializedSession> {
    validate_server_options(&server_options)?;
    let ThriftTransport::SharedMemory(ThriftSharedMemoryTransport { memory_name, .. }) =
        &server_options.thrift_transport
    else {
        return Err(HapiError::Internal(
            "ServerOptions is not configured for shared memory transport".to_owned(),
        ));
    };
    let mem_name_cstr = CString::new(memory_name.clone())?;
    debug!("Connecting to shared memory server: {memory_name:?}");
    let handle = try_connect_with_timeout(
        server_options.connection_retry_interval,
        Duration::from_millis(100),
        || ffi::new_thrift_shared_memory_session(&mem_name_cstr, &server_options.session_info().0),
    )?;
    Ok(UninitializedSession {
        session_handle: Some(handle),
        server_options: Some(server_options),
        server: crate::session::ServerConnection::Borrowed { reported_pid: pid },
    })
}

fn try_connect_with_timeout<F: Fn() -> Result<crate::ffi::raw::HAPI_Session>>(
    timeout: Option<Duration>,
    wait_ms: Duration,
    f: F,
) -> Result<crate::ffi::raw::HAPI_Session> {
    debug!("Trying to connect to server with timeout: {timeout:?}");
    let started = Instant::now();
    let mut last_error = None;
    let handle = loop {
        match f() {
            Ok(handle) => break handle,
            Err(e) => {
                error!("Error while trying to connect to server: {e:?}");
                last_error.replace(e);
            }
        }
        if let Some(timeout) = timeout
            && started.elapsed() >= timeout
        {
            // last_error is guaranteed to be Some() because we break out of the loop if we get a result.
            return Err(last_error.unwrap()).context(format!(
                "Could not connect to server within timeout: {:?}",
                timeout
            ));
        }
        let sleep_for = timeout
            .map(|timeout| wait_ms.min(timeout.saturating_sub(started.elapsed())))
            .unwrap_or(wait_ms);
        if !sleep_for.is_zero() {
            thread::sleep(sleep_for);
        }
    };
    Ok(handle)
}

/// Connect to the Thrift socket server and return an uninitialized session.
pub fn connect_to_socket_server(
    server_options: ServerOptions,
    pid: Option<u32>,
) -> Result<UninitializedSession> {
    validate_server_options(&server_options)?;
    let ThriftTransport::Socket(ThriftSocketTransport { address }) =
        &server_options.thrift_transport
    else {
        return Err(HapiError::Internal(
            "ServerOptions is not configured for socket transport".to_owned(),
        ));
    };
    debug!("Connecting to socket server: {address:?}");
    let host = CString::new(address.ip().to_string())
        .map_err(HapiError::from)
        .context("Converting SocketAddr to CString")?;
    let handle = try_connect_with_timeout(
        server_options.connection_retry_interval,
        Duration::from_millis(100),
        || {
            ffi::new_thrift_socket_session(
                i32::from(address.port()),
                &host,
                &server_options.session_info().0,
            )
        },
    )?;
    Ok(UninitializedSession {
        session_handle: Some(handle),
        server_options: Some(server_options),
        server: crate::session::ServerConnection::Borrowed { reported_pid: pid },
    })
}

/// Start HARS and return its process ID.
///
/// This is a low-level unmanaged API. The caller is responsible for closing
/// all client sessions and, on Unix, reaping the returned child process.
/// [`crate::session::new_thrift_session`] manages those responsibilities.
pub fn start_engine_server(server_options: &ServerOptions) -> Result<u32> {
    validate_server_options(server_options)?;
    let env_variables = server_options.env_variables.as_ref().map(|env_variables| {
        env_variables
            .iter()
            .map(|(k, v)| (k.as_os_str(), v.as_os_str()))
            .collect::<Vec<_>>()
    });
    match &server_options.thrift_transport {
        ThriftTransport::SharedMemory(transport) => {
            debug!(
                "Starting shared memory server name: {}",
                transport.memory_name
            );
            let memory_name = CString::new(transport.memory_name.clone())?;
            ffi::clear_connection_error()?;
            call_with_temp_environment(env_variables.as_deref(), || {
                ffi::start_thrift_shared_memory_server(
                    &memory_name,
                    &server_options.thrift_options().0,
                    server_options.log_file.as_deref(),
                )
                .with_context(|| {
                    format!(
                        "Failed to start shared memory server: {}",
                        transport.memory_name
                    )
                })
            })
        }
        ThriftTransport::Pipe(transport) => {
            debug!(
                "Starting named pipe server: {}",
                transport.pipe_path.display()
            );
            let pipe_name = utils::path_to_cstring(&transport.pipe_path)?;
            ffi::clear_connection_error()?;
            call_with_temp_environment(env_variables.as_deref(), || {
                ffi::start_thrift_pipe_server(
                    &pipe_name,
                    &server_options.thrift_options().0,
                    server_options.log_file.as_deref(),
                )
                .with_context(|| {
                    format!(
                        "Failed to start pipe server: {}",
                        transport.pipe_path.display()
                    )
                })
            })
        }
        ThriftTransport::Socket(transport) => {
            debug!(
                "Starting socket server on port: {}",
                transport.address.port()
            );
            ffi::clear_connection_error()?;
            call_with_temp_environment(env_variables.as_deref(), || {
                ffi::start_thrift_socket_server(
                    i32::from(transport.address.port()),
                    &server_options.thrift_options().0,
                    server_options.log_file.as_deref(),
                )
            })
        }
    }
}

fn validate_server_options(server_options: &ServerOptions) -> Result<()> {
    if server_options.connection_count < 0 {
        return Err(HapiError::Internal(
            "ServerOptions.connection_count cannot be negative".to_owned(),
        ));
    }
    match &server_options.thrift_transport {
        ThriftTransport::SharedMemory(transport) => {
            if transport.memory_name.is_empty() {
                return Err(HapiError::Internal(
                    "Shared-memory transport name cannot be empty".to_owned(),
                ));
            }
            if transport.buffer_size <= 0 {
                return Err(HapiError::Internal(
                    "Shared-memory buffer size must be positive".to_owned(),
                ));
            }
        }
        ThriftTransport::Pipe(transport) if transport.pipe_path.as_os_str().is_empty() => {
            return Err(HapiError::Internal(
                "Pipe transport path cannot be empty".to_owned(),
            ));
        }
        ThriftTransport::Socket(transport) if transport.address.port() == 0 => {
            return Err(HapiError::Internal(
                "Socket transport port cannot be zero".to_owned(),
            ));
        }
        ThriftTransport::Pipe(_) | ThriftTransport::Socket(_) => {}
    }
    Ok(())
}

/// Finish an owned HARS process. During construction failure the process is
/// terminated immediately. During normal auto-close shutdown it first gets a
/// chance to exit after the final client disconnects.
pub(crate) fn finish_owned_server(
    pid: u32,
    auto_close: bool,
    construction_failed: bool,
    pipe_path: Option<&Path>,
) -> Result<()> {
    if !construction_failed && !auto_close {
        return Ok(());
    }

    #[cfg(unix)]
    reap_unix_child(pid, !construction_failed)?;

    #[cfg(windows)]
    finish_windows_child(pid, !construction_failed)?;

    #[cfg(not(any(unix, windows)))]
    let _ = (pid, auto_close, construction_failed);

    if let Some(pipe_path) = pipe_path {
        let _ = std::fs::remove_file(pipe_path);
    }
    Ok(())
}

#[cfg(windows)]
fn finish_windows_child(pid: u32, allow_graceful_exit: bool) -> Result<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{
            OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess,
            WaitForSingleObject,
        },
    };

    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, 0, pid) };
    if handle.is_null() {
        let error = std::io::Error::last_os_error();
        return if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            // The process has already exited.
            Ok(())
        } else {
            Err(HapiError::Io(error))
        };
    }
    let wait_ms = if allow_graceful_exit { 2_000 } else { 0 };
    let wait_result = unsafe { WaitForSingleObject(handle, wait_ms) };
    let result = match wait_result {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_TIMEOUT => {
            if unsafe { TerminateProcess(handle, 1) } == 0 {
                Err(HapiError::Io(std::io::Error::last_os_error()))
            } else {
                let terminated = unsafe { WaitForSingleObject(handle, 1_000) };
                if terminated == WAIT_OBJECT_0 {
                    Ok(())
                } else {
                    Err(HapiError::Internal(format!(
                        "Timed out waiting for HARS process {pid} to terminate"
                    )))
                }
            }
        }
        _ => Err(HapiError::Io(std::io::Error::last_os_error())),
    };
    unsafe {
        CloseHandle(handle);
    }
    result
}

#[cfg(unix)]
fn reap_unix_child(pid: u32, allow_graceful_exit: bool) -> Result<()> {
    let pid = i32::try_from(pid)
        .map_err(|_| HapiError::Internal("HARS PID does not fit pid_t".to_owned()))?;
    let graceful_deadline = Instant::now()
        + if allow_graceful_exit {
            Duration::from_secs(2)
        } else {
            Duration::ZERO
        };

    loop {
        let mut status = 0;
        let result = unsafe { libc::waitpid(pid, &raw mut status, libc::WNOHANG) };
        if result == pid
            || (result == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD))
        {
            return Ok(());
        }
        if result == -1 {
            return Err(HapiError::Io(std::io::Error::last_os_error()));
        }
        if Instant::now() >= graceful_deadline {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    // waitpid returned zero, so this is still our live child and not a reused PID.
    if unsafe { libc::kill(pid, libc::SIGTERM) } == -1 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(HapiError::Io(error));
        }
    }
    let term_deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let mut status = 0;
        let result = unsafe { libc::waitpid(pid, &raw mut status, libc::WNOHANG) };
        if result == pid
            || (result == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD))
        {
            return Ok(());
        }
        if result == -1 {
            return Err(HapiError::Io(std::io::Error::last_os_error()));
        }
        if Instant::now() >= term_deadline {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    if unsafe { libc::kill(pid, libc::SIGKILL) } == -1 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(HapiError::Io(error));
        }
    }
    let mut status = 0;
    let result = unsafe { libc::waitpid(pid, &raw mut status, 0) };
    if result == pid
        || (result == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD))
    {
        Ok(())
    } else {
        Err(HapiError::Io(std::io::Error::last_os_error()))
    }
}

/// Start an interactive Houdini session with engine server embedded.
pub fn start_houdini_server(
    pipe_name: impl AsRef<str>,
    houdini_executable: impl AsRef<Path>,
    fx_license: bool,
    env_variables: Option<&[(String, String)]>,
) -> Result<Child> {
    let mut command = Command::new(houdini_executable.as_ref());
    call_with_temp_environment(env_variables, move || {
        command
            .arg(format!("-hess=pipe:{}", pipe_name.as_ref()))
            .arg(if fx_license {
                "-force-fx-license"
            } else {
                "-core"
            })
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(HapiError::from)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::{ffi::OsString, net::Ipv4Addr, num::NonZeroU64};

    use crate::ffi::enums::StatusVerbosity;

    #[test]
    fn license_preference_display_strings() {
        assert_eq!(
            LicensePreference::AnyAvailable.to_string(),
            "--check-licenses=Houdini-Engine,Houdini-Escape,Houdini-Fx"
        );
        assert_eq!(
            LicensePreference::HoudiniEngineOnly.to_string(),
            "--check-licenses=Houdini-Engine --skip-licenses=Houdini-Escape,Houdini-Fx"
        );
        assert_eq!(
            LicensePreference::HoudiniEngineAndCore.to_string(),
            "--check-licenses=Houdini-Engine,Houdini-Escape --skip-licenses=Houdini-Fx"
        );
    }

    #[test]
    fn shared_memory_transport_builder_applies_options() {
        let transport = ThriftSharedMemoryTransportBuilder::default()
            .with_memory_name("test-memory")
            .with_buffer_type(ThriftSharedMemoryBufferType::RingBuffer)
            .with_buffer_size(NonZeroU64::new(512).unwrap())
            .build();

        assert_eq!(transport.memory_name, "test-memory");
        assert_eq!(
            transport.buffer_type,
            ThriftSharedMemoryBufferType::RingBuffer
        );
        assert_eq!(transport.buffer_size, 512);
    }

    #[test]
    fn shared_memory_transport_builder_clamps_oversized_buffer() {
        let transport = ThriftSharedMemoryTransportBuilder::default()
            .with_buffer_size(NonZeroU64::new(i64::MAX as u64 + 1).unwrap())
            .build();

        assert_eq!(transport.buffer_size, 100);
    }

    #[test]
    fn server_options_shared_memory_maps_to_session_and_thrift_options() {
        let transport = ThriftSharedMemoryTransportBuilder::default()
            .with_buffer_type(ThriftSharedMemoryBufferType::RingBuffer)
            .with_buffer_size(NonZeroU64::new(256).unwrap())
            .build();
        let options = ServerOptions::default()
            .with_auto_close(false)
            .with_verbosity(StatusVerbosity::Statusverbosity2)
            .with_server_ready_timeout(5_000)
            .with_thrift_transport(ThriftTransport::SharedMemory(transport.clone()));

        let session_info = options.session_info();
        assert_eq!(
            session_info.shared_memory_buffer_type(),
            ThriftSharedMemoryBufferType::RingBuffer
        );
        assert_eq!(session_info.shared_memory_buffer_size(), 256);

        let thrift_options = options.thrift_options();
        assert!(!thrift_options.auto_close());
        assert_eq!(
            thrift_options.verbosity(),
            StatusVerbosity::Statusverbosity2
        );
        assert_eq!(
            thrift_options.shared_memory_buffer_type(),
            ThriftSharedMemoryBufferType::RingBuffer
        );
        assert_eq!(thrift_options.shared_memory_buffer_size(), 256);
        assert_eq!(thrift_options.timeout_ms(), 5_000.0);
    }

    #[test]
    fn server_options_license_preference_sets_plugin_env() {
        let options =
            ServerOptions::default().with_license_preference(LicensePreference::HoudiniEngineOnly);
        let env = options.env_variables.expect("env map");
        assert_eq!(
            env.get(&OsString::from("HOUDINI_PLUGIN_LIC_OPT")),
            Some(&OsString::from(
                "--check-licenses=Houdini-Engine --skip-licenses=Houdini-Escape,Houdini-Fx"
            ))
        );
    }

    #[test]
    fn environment_variables_merge_with_license_preference() {
        let options = ServerOptions::default()
            .with_license_preference(LicensePreference::HoudiniEngineOnly)
            .with_env_variables([("HAPI_RS_TEST", "present")].iter());
        let env = options.env_variables.expect("env map");
        assert_eq!(
            env.get(&OsString::from("HAPI_RS_TEST")),
            Some(&OsString::from("present"))
        );
        assert!(env.contains_key(&OsString::from("HOUDINI_PLUGIN_LIC_OPT")));
    }

    #[test]
    fn invalid_public_server_options_are_rejected() {
        let mut options = ServerOptions::shared_memory_with_defaults();
        options.connection_count = -1;
        assert!(validate_server_options(&options).is_err());

        let mut options = ServerOptions::shared_memory_with_defaults();
        let ThriftTransport::SharedMemory(transport) = &mut options.thrift_transport else {
            unreachable!()
        };
        transport.buffer_size = 0;
        assert!(validate_server_options(&options).is_err());

        let mut options = ServerOptions::shared_memory_with_defaults();
        let ThriftTransport::SharedMemory(transport) = &mut options.thrift_transport else {
            unreachable!()
        };
        transport.memory_name.clear();
        assert!(validate_server_options(&options).is_err());

        let options = ServerOptions::default().with_thrift_transport(ThriftTransport::Pipe(
            ThriftPipeTransport {
                pipe_path: PathBuf::new(),
            },
        ));
        assert!(validate_server_options(&options).is_err());

        let options =
            ServerOptions::socket_with_defaults(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));
        assert!(validate_server_options(&options).is_err());
    }

    #[test]
    fn socket_with_defaults_preserves_address() {
        let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 12_345);
        let options = ServerOptions::socket_with_defaults(address);
        let ThriftTransport::Socket(ThriftSocketTransport { address: actual }) =
            options.thrift_transport
        else {
            panic!("expected socket transport");
        };
        assert_eq!(actual, address);
    }

    #[test]
    fn connect_rejects_mismatched_transport_without_calling_hapi() {
        let memory_options = ServerOptions::shared_memory_with_defaults();
        let pipe_options = ServerOptions::pipe_with_defaults();
        let socket_options =
            ServerOptions::socket_with_defaults(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9_999));

        assert!(connect_to_memory_server(pipe_options.clone(), None).is_err());
        assert!(connect_to_pipe_server(memory_options.clone(), None).is_err());
        assert!(connect_to_socket_server(memory_options, None).is_err());
        assert!(connect_to_memory_server(socket_options.clone(), None).is_err());
        assert!(connect_to_pipe_server(socket_options, None).is_err());
    }

    #[test]
    fn zero_connection_timeout_attempts_once_without_sleeping() {
        let attempts = Cell::new(0);
        let started = Instant::now();
        let error = try_connect_with_timeout(Some(Duration::ZERO), Duration::from_secs(1), || {
            attempts.set(attempts.get() + 1);
            Err(HapiError::Internal("not ready".to_owned()))
        })
        .unwrap_err();

        assert_eq!(attempts.get(), 1);
        assert!(started.elapsed() < Duration::from_millis(100));
        assert!(error.to_string().contains("within timeout"));
    }

    #[test]
    fn connection_retry_returns_first_success() {
        let attempts = Cell::new(0);
        let handle = try_connect_with_timeout(None, Duration::ZERO, || {
            attempts.set(attempts.get() + 1);
            if attempts.get() < 3 {
                Err(HapiError::Internal("not ready".to_owned()))
            } else {
                Ok(crate::ffi::raw::HAPI_Session {
                    type_: crate::ffi::raw::SessionType::Thrift,
                    id: 42,
                })
            }
        })
        .unwrap();

        assert_eq!(attempts.get(), 3);
        assert_eq!(handle.id, 42);
    }

    #[cfg(unix)]
    #[test]
    fn owned_child_is_reaped_after_graceful_exit() {
        let child = Command::new("sh")
            .args(["-c", "exit 0"])
            .spawn()
            .expect("spawn child");
        let pid = child.id();
        drop(child);

        reap_unix_child(pid, true).expect("reap child");
        assert_child_already_reaped(pid);
    }

    #[cfg(unix)]
    #[test]
    fn owned_child_uses_kill_fallback_when_term_is_ignored() {
        let child = Command::new("sh")
            .args(["-c", "trap '' TERM; exec sleep 30"])
            .spawn()
            .expect("spawn child");
        let pid = child.id();
        drop(child);
        thread::sleep(Duration::from_millis(100));

        reap_unix_child(pid, false).expect("terminate and reap child");
        assert_child_already_reaped(pid);
    }

    #[cfg(unix)]
    #[test]
    fn auto_close_false_leaves_owned_server_running() {
        let mut child = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn child");
        let pid = child.id();

        finish_owned_server(pid, false, false, None).expect("leave child running");
        assert!(child.try_wait().expect("query child").is_none());

        child.kill().expect("kill test child");
        child.wait().expect("reap test child");
    }

    #[cfg(unix)]
    #[test]
    fn construction_failure_terminates_child_and_removes_owned_pipe() {
        let child = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn child");
        let pid = child.id();
        drop(child);
        let pipe = tempfile::NamedTempFile::new().expect("temporary pipe placeholder");
        let pipe_path = pipe.path().to_owned();
        drop(pipe);

        finish_owned_server(pid, true, true, Some(&pipe_path)).expect("rollback server");
        assert!(!pipe_path.exists());
        assert_child_already_reaped(pid);
    }

    #[cfg(unix)]
    #[test]
    fn already_reaped_child_is_a_successful_noop() {
        let mut child = Command::new("sh")
            .args(["-c", "exit 0"])
            .spawn()
            .expect("spawn child");
        let pid = child.id();
        child.wait().expect("reap child");

        reap_unix_child(pid, true).expect("already-reaped child is okay");
    }

    #[cfg(unix)]
    fn assert_child_already_reaped(pid: u32) {
        let mut status = 0;
        let result = unsafe { libc::waitpid(pid as i32, &raw mut status, libc::WNOHANG) };
        assert_eq!(result, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
}
