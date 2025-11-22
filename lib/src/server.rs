use std::{
    ffi::{CString, OsString},
    net::SocketAddrV4,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use log::debug;
use temp_env;

use crate::{
    errors::{ErrorContext, HapiError, Result},
    ffi::{self, ThriftServerOptions, enums::StatusVerbosity},
    session::{ConnectionType, Session, SessionOptions},
    utils,
};

pub use crate::ffi::raw::ThriftSharedMemoryBufferType;

#[derive(Clone, Debug)]
pub struct SharedMemoryTransport {
    pub memory_name: String,
}

impl SharedMemoryTransport {
    fn new_random() -> Self {
        Self {
            memory_name: format!("shared-memory-{}", utils::random_string(16)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SocketTransport {
    pub address: SocketAddrV4,
}

#[derive(Clone, Debug)]
pub struct PipeTransport {
    pub pipe_path: PathBuf,
}

#[derive(Clone, Debug)]
pub enum ThriftTransport {
    SharedMemory(SharedMemoryTransport),
    Pipe(PipeTransport),
    Socket(SocketTransport),
}

pub struct ServerOptions {
    transport: ThriftTransport,
    thrift_options: ThriftServerOptions,
    log_file: Option<CString>,
    env_variables: Option<Vec<(OsString, OsString)>>,
    pub(crate) connection_timeout: Option<Duration>,
}

impl ServerOptions {
    /// Create options for a shared-memory transport with a random name.
    pub fn shared_memory() -> Self {
        Self::from_transport(ThriftTransport::SharedMemory(
            SharedMemoryTransport::new_random(),
        ))
    }

    /// Create options for a named pipe transport.
    pub fn pipe(path: impl Into<PathBuf>) -> Self {
        Self::from_transport(ThriftTransport::Pipe(PipeTransport {
            pipe_path: path.into(),
        }))
    }

    /// Create options for a socket transport.
    pub fn socket(address: SocketAddrV4) -> Self {
        Self::from_transport(ThriftTransport::Socket(SocketTransport { address }))
    }

    fn from_transport(transport: ThriftTransport) -> Self {
        Self {
            transport,
            thrift_options: ThriftServerOptions::default()
                .with_timeout_ms(4000f32)
                .with_verbosity(StatusVerbosity::Statusverbosity0),
            log_file: None,
            env_variables: None,
            connection_timeout: None,
        }
    }

    pub fn transport(&self) -> &ThriftTransport {
        &self.transport
    }

    /// Replace the transport configuration.
    pub fn with_transport(mut self, transport: ThriftTransport) -> Self {
        self.transport = transport;
        self
    }

    /// Provide a custom shared memory name. Only used if transport is [`ThriftTransport::SharedMemory`].
    pub fn memory_name(mut self, name: impl Into<String>) -> Self {
        if let ThriftTransport::SharedMemory(ref mut cfg) = self.transport {
            cfg.memory_name = name.into();
        }
        self
    }

    /// Override the pipe path. Only used if transport is [`ThriftTransport::Pipe`].
    pub fn pipe_path(mut self, path: impl Into<PathBuf>) -> Self {
        if let ThriftTransport::Pipe(ref mut cfg) = self.transport {
            cfg.pipe_path = path.into();
        }
        self
    }

    /// Override the socket address used to connect to the server. Only used if transport is [`ThriftTransport::Socket`].
    pub fn socket_address(mut self, addr: SocketAddrV4) -> Self {
        if let ThriftTransport::Socket(ref mut cfg) = self.transport {
            cfg.address = addr;
        }
        self
    }

    /// Set a connection timeout used when establishing Thrift sessions.
    pub fn connection_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.connection_timeout = timeout;
        self
    }

    /// Set the thrift server options. See [`ThriftServerOptions`] for more details.
    pub fn with_thrift_options(mut self, options: ThriftServerOptions) -> Self {
        self.thrift_options = options;
        self
    }

    /// Mutate the underlying [`ThriftServerOptions`] via a closure.
    pub fn configure_thrift(mut self, configure: impl FnOnce(&mut ThriftServerOptions)) -> Self {
        configure(&mut self.thrift_options);
        self
    }

    /// Set the log file for the server.
    pub fn with_log_file(mut self, file: impl AsRef<Path>) -> Self {
        self.log_file = Some(utils::path_to_cstring(file).expect("Path to CString failed"));
        self
    }

    /// Set **real** environment variables before the server starts.
    /// Unlike [Session::set_server_var()], where the variables are set in the session, after the server starts.
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
    pub fn auto_close(mut self, auto_close: bool) -> Self {
        self.thrift_options.set_auto_close(auto_close);
        self
    }

    fn run_with_environment<R, F: FnOnce() -> Result<R>>(&self, f: F) -> Result<R> {
        if let Some(env_variables) = &self.env_variables {
            let env_variables = env_variables
                .iter()
                .map(|(k, v)| (k, Some(v)))
                .collect::<Vec<_>>();
            temp_env::with_vars(env_variables.as_slice(), f)
        } else {
            f()
        }
    }
}

/// Connect to the engine process via a pipe file.
/// If `timeout` is Some, function will try to connect to
/// the server multiple times every 100ms until `timeout` is reached.
pub fn connect_to_pipe_server(
    pipe: impl AsRef<Path>,
    session_options: SessionOptions,
    timeout: Option<Duration>,
    pid: Option<u32>,
) -> Result<Session> {
    debug!("Connecting to Thrift session: {:?}", pipe.as_ref());
    let c_str = utils::path_to_cstring(&pipe)?;
    let pipe = pipe.as_ref().as_os_str().to_os_string();
    let timeout = timeout.unwrap_or_default();
    let mut waited = Duration::from_secs(0);
    let wait_ms = Duration::from_millis(100);
    let handle = loop {
        let mut last_error = None;
        debug!("Trying to connect to pipe server");
        match ffi::new_thrift_piped_session(&c_str, &session_options.session_info.0) {
            Ok(handle) => break handle,
            Err(e) => {
                last_error.replace(e);
                thread::sleep(wait_ms);
                waited += wait_ms;
            }
        }
        if waited > timeout {
            // last_error is guaranteed to be Some().
            return Err(last_error.unwrap()).context("Connection timeout");
        }
    };
    let connection = ConnectionType::ThriftPipe(pipe);
    let session = Session::new(handle, connection, session_options, pid);
    session.initialize()?;
    Ok(session)
}

pub fn connect_to_memory_server(
    memory_name: &str,
    session_options: SessionOptions,
    timeout: Option<Duration>,
    pid: Option<u32>,
) -> Result<Session> {
    debug!("Connecting to shared memory session: {memory_name}");
    let mem_name = String::from(memory_name);
    let mem_name_cstr = CString::new(memory_name)?;
    let timeout = timeout.unwrap_or_default();
    let mut waited = Duration::from_secs(0);
    let wait_ms = Duration::from_millis(100);
    let handle = loop {
        let mut last_error = None;
        match ffi::new_thrift_shared_memory_session(&mem_name_cstr, &session_options.session_info.0)
        {
            Ok(handle) => break handle,
            Err(e) => {
                last_error.replace(e);
                thread::sleep(wait_ms);
                waited += wait_ms;
            }
        }
        if waited > timeout {
            return Err(last_error.unwrap()).context("Connection timeout");
        }
    };

    let connection = ConnectionType::SharedMemory(mem_name);
    let session = Session::new(handle, connection, session_options, pid);
    session.initialize()?;
    Ok(session)
}

/// Connect to the engine process via a Unix socket
pub fn connect_to_socket_server(
    addr: SocketAddrV4,
    session_options: SessionOptions,
    pid: Option<u32>,
) -> Result<Session> {
    debug!("Connecting to socket server: {:?}", addr);
    let host = CString::new(addr.ip().to_string()).expect("SocketAddr->CString");
    let handle =
        ffi::new_thrift_socket_session(addr.port() as i32, &host, &session_options.session_info.0)?;
    let connection = ConnectionType::ThriftSocket(addr);
    let session = Session::new(handle, connection, session_options, pid);
    session.initialize()?;
    Ok(session)
}

/// Spawn a new pipe Engine process and return its PID
pub fn start_engine_pipe_server(
    path: impl AsRef<Path>,
    server_options: &ServerOptions,
) -> Result<u32> {
    debug!("Starting named pipe server: {:?}", path.as_ref());
    let pipe_name = utils::path_to_cstring(path)?;
    ffi::clear_connection_error()?;
    server_options.run_with_environment(|| {
        ffi::start_thrift_pipe_server(
            &pipe_name,
            &server_options.thrift_options.0,
            server_options.log_file.as_deref(),
        )
    })
}

/// Spawn a new socket Engine server and return its PID
pub fn start_engine_socket_server(port: u16, server_options: &ServerOptions) -> Result<u32> {
    debug!("Starting socket server on port: {}", port);
    ffi::clear_connection_error()?;
    let timeout = server_options.connection_timeout.unwrap_or_default();
    let wait_ms = Duration::from_millis(100);
    let mut waited = Duration::from_secs(0);
    loop {
        match server_options.run_with_environment(|| {
            ffi::start_thrift_socket_server(
                port as i32,
                &server_options.thrift_options.0,
                server_options.log_file.as_deref(),
            )
        }) {
            Ok(pid) => break Ok(pid),
            Err(e) => {
                if timeout.is_zero() || waited > timeout {
                    break Err(e).context("Could not start socket server");
                }
                thread::sleep(wait_ms);
                waited += wait_ms;
                if waited > timeout {
                    break Err(e).context("Socket server start timeout");
                }
            }
        }
    }
}

/// Spawn a new Engine server utilizing shared memory to transfer data.
pub fn start_engine_shared_memory_server(
    memory_name: &str,
    server_options: &ServerOptions,
) -> Result<u32> {
    debug!("Starting shared memory server name: {memory_name}");
    let memory_name = CString::new(memory_name)?;
    ffi::clear_connection_error()?;
    server_options.run_with_environment(|| {
        ffi::start_thrift_shared_memory_server(
            &memory_name,
            &server_options.thrift_options.0,
            server_options.log_file.as_deref(),
        )
    })
}

/// Start an interactive Houdini session with engine server embedded.
pub fn start_houdini_server(
    pipe_name: impl AsRef<str>,
    houdini_executable: impl AsRef<Path>,
    fx_license: bool,
) -> Result<Child> {
    Command::new(houdini_executable.as_ref())
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
}
