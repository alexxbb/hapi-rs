use std::{
    collections::HashMap,
    ffi::{CString, OsStr, OsString},
    net::SocketAddrV4,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use log::{debug, error};
use temp_env;

use crate::{
    errors::{ErrorContext, HapiError, Result},
    ffi::{self, ThriftServerOptions, enums::StatusVerbosity},
    session::UninitializedSession,
    utils,
};

pub use crate::ffi::raw::ThriftSharedMemoryBufferType;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LicensePreference {
    AnyAvailable,
    HoudiniEngineOnly,
    HoudiniEngineAndCore,
}

impl ToString for LicensePreference {
    fn to_string(&self) -> String {
        match self {
            LicensePreference::AnyAvailable => {
                "--check-licenses=Houdini-Engine,Houdini-Escape,Houdini-Fx".to_owned()
            }
            LicensePreference::HoudiniEngineOnly => {
                "--check-licenses=Houdini-Engine --skip-licenses=Houdini-Escape,Houdini-Fx"
                    .to_owned()
            }
            LicensePreference::HoudiniEngineAndCore => {
                "--check-licenses=Houdini-Engine,Houdini-Escape --skip-licenses=Houdini-Fx"
                    .to_owned()
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ThriftSharedMemoryTransport {
    pub memory_name: String,
    pub buffer_type: ThriftSharedMemoryBufferType,
    pub buffer_size: i64,
}

#[derive(Clone, Debug)]
pub struct ThriftSocketTransport {
    pub address: SocketAddrV4,
}

#[derive(Clone, Debug)]
pub struct ThriftPipeTransport {
    pub pipe_path: PathBuf,
}

#[derive(Clone, Debug)]
pub enum ThriftTransport {
    SharedMemory(ThriftSharedMemoryTransport),
    Pipe(ThriftPipeTransport),
    Socket(ThriftSocketTransport),
}

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
            buffer_size: 1024, // MB
        }
    }
}

impl ThriftSharedMemoryTransportBuilder {
    pub fn with_memory_name(mut self, name: impl Into<String>) -> Self {
        self.memory_name = name.into();
        self
    }
    pub fn with_buffer_type(mut self, buffer_type: ThriftSharedMemoryBufferType) -> Self {
        self.buffer_type = buffer_type;
        self
    }
    pub fn with_buffer_size(mut self, buffer_size: i64) -> Self {
        self.buffer_size = buffer_size;
        self
    }
    pub fn build(self) -> ThriftSharedMemoryTransport {
        ThriftSharedMemoryTransport {
            memory_name: self.memory_name,
            buffer_type: self.buffer_type,
            buffer_size: self.buffer_size,
        }
    }
}

// TODO: rename ServerConfiguration
#[derive(Clone, Debug)]
pub struct ServerOptions {
    pub thrift_transport: ThriftTransport,
    pub auto_close: bool,
    pub verbosity: StatusVerbosity,
    pub log_file: Option<CString>,
    pub env_variables: Option<HashMap<OsString, OsString>>,
    pub license_preference: Option<LicensePreference>,
    pub(crate) connection_retry_interval: Option<Duration>,
}

impl ServerOptions {
    /// Create options for a shared-memory transport with a random name.
    pub fn shared_memory_with_defaults() -> Self {
        Self {
            thrift_transport: ThriftTransport::SharedMemory(
                ThriftSharedMemoryTransportBuilder::default().build(),
            ),
            auto_close: true,
            verbosity: StatusVerbosity::Statusverbosity0,
            log_file: None,
            env_variables: None,
            license_preference: None,
            connection_retry_interval: Some(Duration::from_secs(10)),
        }
    }

    /// Create options for a named pipe transport.
    pub fn pipe_with_defaults() -> Self {
        Self {
            thrift_transport: ThriftTransport::Pipe(ThriftPipeTransport {
                pipe_path: PathBuf::from(format!("hapi-pipe-{}", utils::random_string(16))),
            }),
            auto_close: true,
            verbosity: StatusVerbosity::Statusverbosity0,
            log_file: None,
            env_variables: None,
            license_preference: None,
            connection_retry_interval: Some(Duration::from_secs(10)),
        }
    }

    /// Create options for a socket transport.
    pub fn socket_with_defaults(address: SocketAddrV4) -> Self {
        Self {
            thrift_transport: ThriftTransport::Socket(ThriftSocketTransport { address }),
            auto_close: true,
            verbosity: StatusVerbosity::Statusverbosity0,
            log_file: None,
            env_variables: None,
            license_preference: None,
            connection_retry_interval: Some(Duration::from_secs(10)),
        }
    }

    pub(crate) fn session_info(&self) -> crate::ffi::SessionInfo {
        // FIXME: connection_count should be configurable!
        // It's set to 0 because of a bug in HARS which prevents session creation if the value is > 0.
        // However, async attribute access requires a connection count > 0 according to SESI support, otherwise HARS crashes too.
        let mut session_info = crate::ffi::SessionInfo::default().with_connection_count(0);

        if let ThriftTransport::SharedMemory(transport) = &self.thrift_transport {
            session_info.set_shared_memory_buffer_type(transport.buffer_type);
            session_info.set_shared_memory_buffer_size(transport.buffer_size);
        }

        session_info
    }

    pub fn thrift_transport(&self) -> &ThriftTransport {
        &self.thrift_transport
    }

    pub(crate) fn thrift_options(&self) -> crate::ffi::ThriftServerOptions {
        let mut options = ThriftServerOptions::default()
            .with_auto_close(self.auto_close)
            .with_verbosity(self.verbosity);

        if let ThriftTransport::SharedMemory(transport) = &self.thrift_transport {
            options.set_shared_memory_buffer_type(transport.buffer_type);
            options.set_shared_memory_buffer_size(transport.buffer_size);
        }

        options
    }

    pub fn with_thrift_transport(transport: ThriftTransport) -> Self {
        Self {
            thrift_transport: transport,
            auto_close: true,
            verbosity: StatusVerbosity::Statusverbosity0,
            log_file: None,
            env_variables: None,
            license_preference: None,
            connection_retry_interval: Some(Duration::from_secs(10)),
        }
    }

    /// Set a connection timeout used when establishing Thrift sessions.
    pub fn with_connection_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.connection_retry_interval = timeout;
        self
    }

    /// Set the license preference for the server.
    /// For more information, see https://www.sidefx.com/docs/houdini//licensing/system.html
    /// Default is No preference, the server decides which license to check out.
    pub fn with_license_preference(mut self, license_preference: LicensePreference) -> Self {
        self.license_preference.replace(license_preference);

        if let Some(license_preference) = self.license_preference {
            self.env_variables.as_mut().map(|env_variables| {
                env_variables.insert(
                    OsString::from("HOUDINI_PLUGIN_LIC_OPT"),
                    OsString::from(license_preference.to_string()),
                )
            });
        }

        self
    }

    /// Set the log file for the server.
    /// BUG: HARS 21.0.685 has a bug where the log file is always created in the working directory
    pub fn with_log_file(mut self, file: impl AsRef<Path>) -> Self {
        self.log_file = Some(utils::path_to_cstring(file).expect("Path to CString failed"));
        self
    }

    /// Set **real** environment variables before the server starts.
    /// Unlike [`crate::session::Session::set_server_var`], where the variables are set in the session after the
    /// server starts.
    pub fn with_env_variables<'a, I, K, V>(mut self, variables: I) -> Self
    where
        I: Iterator<Item = &'a (K, V)>,
        K: Into<OsString> + Clone + 'a,
        V: Into<OsString> + Clone + 'a,
    {
        self.env_variables = Some(
            variables
                .map(|(k, v)| (k.clone().into(), v.clone().into()))
                .collect(),
        );
        self
    }

    /// Automatically close the server when the last connection drops.
    pub fn with_auto_close(mut self, auto_close: bool) -> Self {
        self.auto_close = auto_close;
        self
    }

    fn run_with_environment<R, T: AsRef<OsStr>, F: FnOnce() -> Result<R>>(
        variables: Option<&[(T, T)]>,
        f: F,
    ) -> Result<R> {
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
}

/// Connect to the Thrift pipe server and return an uninitialized session.
pub fn connect_to_pipe_server(
    server_options: ServerOptions,
    pid: Option<u32>,
) -> Result<UninitializedSession> {
    let ThriftTransport::Pipe(ThriftPipeTransport { pipe_path }) =
        server_options.thrift_transport()
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
        session_handle: handle,
        server_options: Some(server_options),
        server_pid: pid,
    })
}

/// Connect to the Thrift shared memory server and return an uninitialized session.
pub fn connect_to_memory_server(
    server_options: ServerOptions,
    pid: Option<u32>,
) -> Result<UninitializedSession> {
    let ThriftTransport::SharedMemory(ThriftSharedMemoryTransport { memory_name, .. }) =
        server_options.thrift_transport()
    else {
        return Err(HapiError::Internal(
            "ServerOptions is not configured for shared memory transport".to_owned(),
        ));
    };
    let mem_name_cstr = CString::new(memory_name.clone())?;
    debug!("Connecting to shared memory server: {:?}", memory_name);
    let handle = try_connect_with_timeout(
        server_options.connection_retry_interval,
        Duration::from_millis(100),
        || ffi::new_thrift_shared_memory_session(&mem_name_cstr, &server_options.session_info().0),
    )?;
    Ok(UninitializedSession {
        session_handle: handle,
        server_options: Some(server_options),
        server_pid: pid,
    })
}

fn try_connect_with_timeout<F: Fn() -> Result<crate::ffi::raw::HAPI_Session>>(
    timeout: Option<Duration>,
    wait_ms: Duration,
    f: F,
) -> Result<crate::ffi::raw::HAPI_Session> {
    debug!("Trying to connect to server with timeout: {:?}", timeout);
    let mut waited = Duration::from_secs(0);
    let mut last_error = None;
    let handle = loop {
        match f() {
            Ok(handle) => break handle,
            Err(e) => {
                error!("Error while trying to connect to server: {:?}", e);
                last_error.replace(e);
                thread::sleep(wait_ms);
                waited += wait_ms;
            }
        }
        if let Some(timeout) = timeout
            && waited > timeout
        {
            // last_error is guaranteed to be Some() because we break out of the loop if we get a result.
            return Err(last_error.unwrap()).context(format!(
                "Could not connect to server within timeout: {timeout:?}"
            ));
        }
    };
    Ok(handle)
}

/// Connect to the Thrift socket server and return an uninitialized session.
pub fn connect_to_socket_server(
    server_options: ServerOptions,
    pid: Option<u32>,
) -> Result<UninitializedSession> {
    let ThriftTransport::Socket(ThriftSocketTransport { address }) =
        server_options.thrift_transport()
    else {
        return Err(HapiError::Internal(
            "ServerOptions is not configured for socket transport".to_owned(),
        ));
    };
    debug!("Connecting to socket server: {:?}", address);
    let host = CString::new(address.ip().to_string()).expect("SocketAddr->CString");
    let handle = try_connect_with_timeout(
        server_options.connection_retry_interval,
        Duration::from_millis(100),
        || {
            ffi::new_thrift_socket_session(
                address.port() as i32,
                &host,
                &server_options.session_info().0,
            )
        },
    )?;
    Ok(UninitializedSession {
        session_handle: handle,
        server_options: Some(server_options),
        server_pid: pid,
    })
}

pub fn start_engine_server(server_options: &ServerOptions) -> Result<u32> {
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
            ServerOptions::run_with_environment(env_variables.as_deref(), || {
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
            debug!("Starting named pipe server: {:?}", transport.pipe_path);
            let pipe_name = utils::path_to_cstring(&transport.pipe_path)?;
            ffi::clear_connection_error()?;
            ServerOptions::run_with_environment(env_variables.as_deref(), || {
                ffi::start_thrift_pipe_server(
                    &pipe_name,
                    &server_options.thrift_options().0,
                    server_options.log_file.as_deref(),
                )
                .with_context(|| format!("Failed to start pipe server: {:?}", transport.pipe_path))
            })
        }
        ThriftTransport::Socket(transport) => {
            debug!(
                "Starting socket server on port: {}",
                transport.address.port()
            );
            ffi::clear_connection_error()?;
            ServerOptions::run_with_environment(env_variables.as_deref(), || {
                ffi::start_thrift_socket_server(
                    transport.address.port() as i32,
                    &server_options.thrift_options().0,
                    server_options.log_file.as_deref(),
                )
            })
        }
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
    ServerOptions::run_with_environment(env_variables, move || {
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
