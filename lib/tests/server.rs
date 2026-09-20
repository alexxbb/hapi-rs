use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpListener},
    num::NonZeroU64,
    path::PathBuf,
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

use hapi_rs::Result;
use hapi_rs::enums::SessionType;
use hapi_rs::server::{
    LicensePreference, ServerOptions, ThriftSharedMemoryBufferType,
    ThriftSharedMemoryTransportBuilder, ThriftTransport, connect_to_memory_server,
    connect_to_pipe_server, connect_to_socket_server, start_engine_server,
};
use hapi_rs::session::{
    License, Session, SessionOptions, UninitializedSession, new_thrift_session,
};
use pretty_assertions::assert_eq;

fn server_options_with_temp_log(base: ServerOptions) -> ServerOptions {
    let temp_log_file = tempfile::NamedTempFile::new().expect("temp log file");
    let temp_log_path = temp_log_file
        .into_temp_path()
        .keep()
        .expect("keep temp log");
    base.with_log_file(temp_log_path)
        .with_license_preference(LicensePreference::HoudiniEngineAndCore)
}

fn unique_name(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    )
}

fn free_local_socket_addr() -> SocketAddrV4 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)
}

fn assert_thrift_session(session: &Session) {
    assert!(session.is_valid());
    assert_eq!(session.session_type(), SessionType::Thrift);
    let pid = session.server_pid().expect("server pid");
    assert!(pid > 0);
}

fn initialize_session(uninitialized: UninitializedSession) -> Result<Session> {
    uninitialized.initialize(SessionOptions::default())
}

fn thrift_session_from_start_and_connect<F>(
    server_options: ServerOptions,
    connect: F,
) -> Result<Session>
where
    F: FnOnce(ServerOptions, Option<u32>) -> Result<UninitializedSession>,
{
    let pid = start_engine_server(&server_options)?;
    let session = connect(server_options, Some(pid))?;
    initialize_session(session)
}

#[test]
fn shared_memory_server_starts_and_connects() -> Result<()> {
    let server_options = server_options_with_temp_log(ServerOptions::shared_memory_with_defaults());
    let session = new_thrift_session(SessionOptions::default(), server_options)?;
    assert_thrift_session(&session);
    Ok(())
}

#[test]
fn shared_memory_server_ring_buffer_and_custom_size() -> Result<()> {
    let transport = ThriftSharedMemoryTransportBuilder::default()
        .with_memory_name(unique_name("hapi-rs-ring"))
        .with_buffer_type(ThriftSharedMemoryBufferType::RingBuffer)
        .with_buffer_size(NonZeroU64::new(128).unwrap())
        .build();
    let server_options = server_options_with_temp_log(
        ServerOptions::default().with_thrift_transport(ThriftTransport::SharedMemory(transport)),
    );
    let session = thrift_session_from_start_and_connect(server_options, connect_to_memory_server)?;
    assert_thrift_session(&session);
    Ok(())
}

#[test]
fn pipe_server_starts_and_connects() -> Result<()> {
    let pipe_path = PathBuf::from(format!("/tmp/{}", unique_name("hapi-rs-pipe")));
    let server_options =
        server_options_with_temp_log(ServerOptions::pipe_with_defaults().with_thrift_transport(
            ThriftTransport::Pipe(hapi_rs::server::ThriftPipeTransport { pipe_path }),
        ));
    let session = thrift_session_from_start_and_connect(server_options, connect_to_pipe_server)?;
    assert_thrift_session(&session);
    Ok(())
}

#[test]
fn socket_server_starts_and_connects() -> Result<()> {
    let address = free_local_socket_addr();
    let server_options = server_options_with_temp_log(ServerOptions::socket_with_defaults(address));
    let session = thrift_session_from_start_and_connect(server_options, connect_to_socket_server)?;
    assert_thrift_session(&session);
    Ok(())
}

#[test]
fn server_env_variables_visible_in_session() -> Result<()> {
    let server_options = server_options_with_temp_log(
        ServerOptions::pipe_with_defaults()
            .with_env_variables([("HAPI_RS_TEST", "hapi_rs_is_awesome")].iter()),
    );
    let session = new_thrift_session(SessionOptions::default(), server_options)?;
    assert_thrift_session(&session);
    session.set_server_var::<str>("FOO", "foo_string")?;
    assert_eq!(session.get_server_var::<str>("FOO")?, "foo_string");
    session.set_server_var::<i32>("BAR", &123)?;
    assert_eq!(session.get_server_var::<i32>("BAR")?, 123);
    assert!(!session.get_server_variables()?.is_empty());
    assert_eq!(
        session.get_server_var::<str>("HAPI_RS_TEST")?,
        "hapi_rs_is_awesome"
    );
    assert!(!std::env::vars().any(|(k, _)| k == "HAPI_RS_TEST"));
    Ok(())
}

#[test]
fn license_preference() -> Result<()> {
    let server_options = server_options_with_temp_log(
        ServerOptions::shared_memory_with_defaults()
            .with_license_preference(LicensePreference::HoudiniEngineAndCore),
    );
    let session = thrift_session_from_start_and_connect(server_options, connect_to_memory_server)?;
    assert_thrift_session(&session);
    session.create_node("Object/null")?;
    let license_type = session.get_license_type()?;
    assert!([License::LicenseHoudini, License::HoudiniEngine].contains(&license_type));
    Ok(())
}

#[test]
fn connect_to_missing_memory_server_times_out() -> Result<()> {
    let transport = ThriftSharedMemoryTransportBuilder::default()
        .with_memory_name("hapi-rs-nonexistent-memory")
        .build();
    let server_options = ServerOptions::default()
        .with_thrift_transport(ThriftTransport::SharedMemory(transport))
        .with_connection_timeout(Some(Duration::from_millis(300)));

    let err = connect_to_memory_server(server_options, None).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("Could not connect to server within timeout"),
        "unexpected error: {message}"
    );
    Ok(())
}

#[test]
fn connect_rejects_pipe_options_for_memory_connect() -> Result<()> {
    let err = connect_to_memory_server(ServerOptions::pipe_with_defaults(), None).unwrap_err();
    assert!(
        err.to_string()
            .contains("ServerOptions is not configured for shared memory transport")
    );
    Ok(())
}

#[test]
fn explicit_close_is_shared_and_idempotent() -> Result<()> {
    let session = new_thrift_session(
        SessionOptions::default(),
        server_options_with_temp_log(ServerOptions::shared_memory_with_defaults()),
    )?;
    let clone = session.clone();
    let pid = session.server_pid().expect("owned server pid");

    session.close()?;
    assert!(!session.is_valid());
    assert!(!clone.is_valid());
    clone.close()?;
    assert_owned_child_reaped(pid);
    Ok(())
}

#[test]
fn concurrent_final_clone_drops_finalize_once() -> Result<()> {
    let session = new_thrift_session(
        SessionOptions::default(),
        server_options_with_temp_log(ServerOptions::shared_memory_with_defaults()),
    )?;
    let pid = session.server_pid().expect("owned server pid");
    let clone = session.clone();
    let barrier = Arc::new(Barrier::new(3));

    let first_barrier = barrier.clone();
    let first = thread::spawn(move || {
        first_barrier.wait();
        drop(session);
    });
    let second_barrier = barrier.clone();
    let second = thread::spawn(move || {
        second_barrier.wait();
        drop(clone);
    });
    barrier.wait();
    first.join().expect("first drop thread");
    second.join().expect("second drop thread");

    assert_owned_child_reaped(pid);
    Ok(())
}

#[cfg(unix)]
#[test]
fn dropping_uninitialized_session_closes_connection() -> Result<()> {
    let server_options = server_options_with_temp_log(ServerOptions::shared_memory_with_defaults());
    let pid = start_engine_server(&server_options)?;
    let uninitialized = connect_to_memory_server(server_options, None)?;

    drop(uninitialized);

    wait_for_test_child_exit(pid);
    Ok(())
}

#[cfg(unix)]
#[test]
fn managed_auto_close_false_leaves_hars_running() -> Result<()> {
    let session = new_thrift_session(
        SessionOptions::default(),
        server_options_with_temp_log(
            ServerOptions::shared_memory_with_defaults().with_auto_close(false),
        ),
    )?;
    let pid = session.server_pid().expect("owned server pid");

    session.close()?;

    let was_running = unsafe { libc::kill(pid as i32, 0) } == 0;
    terminate_test_child(pid);
    assert!(was_running, "auto_close=false HARS exited unexpectedly");
    Ok(())
}

#[cfg(unix)]
#[test]
fn closing_borrowed_pipe_session_does_not_terminate_external_server() -> Result<()> {
    let pipe_path = PathBuf::from(format!("/tmp/{}", unique_name("hapi-rs-borrowed")));
    let server_options = server_options_with_temp_log(
        ServerOptions::pipe_with_defaults()
            .with_thrift_transport(ThriftTransport::Pipe(
                hapi_rs::server::ThriftPipeTransport {
                    pipe_path: pipe_path.clone(),
                },
            ))
            .with_auto_close(false),
    );
    let pid = start_engine_server(&server_options)?;

    let result = (|| -> Result<bool> {
        let session =
            connect_to_pipe_server(server_options, None)?.initialize(SessionOptions::default())?;
        session.close()?;
        Ok(unsafe { libc::kill(pid as i32, 0) } == 0)
    })();

    terminate_test_child(pid);
    let _ = std::fs::remove_file(&pipe_path);
    assert!(result?, "borrowed HARS process was terminated");
    Ok(())
}

#[cfg(unix)]
fn assert_owned_child_reaped(pid: u32) {
    let mut status = 0;
    let result = unsafe { libc::waitpid(pid as i32, &raw mut status, libc::WNOHANG) };
    assert_eq!(result, -1, "HARS child {pid} was not reaped");
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD),
        "unexpected waitpid result for HARS child {pid}"
    );
}

#[cfg(unix)]
fn wait_for_test_child_exit(pid: u32) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let mut status = 0;
        let result = unsafe { libc::waitpid(pid as i32, &raw mut status, libc::WNOHANG) };
        if result == pid as i32 {
            return;
        }
        assert_eq!(result, 0, "waitpid failed for HARS child {pid}");
        if std::time::Instant::now() >= deadline {
            terminate_test_child(pid);
            panic!("HARS child {pid} did not exit after its uninitialized client was dropped");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(unix)]
fn terminate_test_child(pid: u32) {
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
    let mut status = 0;
    let result = unsafe { libc::waitpid(pid as i32, &raw mut status, 0) };
    assert_eq!(result, pid as i32, "failed to reap test HARS child {pid}");
}

#[cfg(not(unix))]
fn assert_owned_child_reaped(_pid: u32) {}
