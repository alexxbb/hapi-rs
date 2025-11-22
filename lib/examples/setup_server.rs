use std::time::Duration;

use hapi_rs::Result;
use hapi_rs::node::CookOptions;
use hapi_rs::server::*;
use hapi_rs::session::{Session, SessionOptions, new_thrift_session};

fn simple_default_thrift_session() -> Result<Session> {
    new_thrift_session(
        SessionOptions::default(),
        ServerOptions::shared_memory_with_defaults(),
    )
}

fn thirft_advanced_setup() -> Result<Session> {
    let server_options = ServerOptions::with_thrift_transport(ThriftTransport::SharedMemory(
        ThriftSharedMemoryTransportBuilder::default()
            .with_memory_name("hapi-rs-advanced-server")
            .with_buffer_size(1000)
            .with_buffer_type(ThriftSharedMemoryBufferType::Buffer)
            .build(),
    ))
    .with_connection_timeout(Some(Duration::from_secs(10)))
    .with_log_file(std::env::temp_dir().join("hapi-rs-advanced-server.log"))
    .with_env_variables(vec![("HAPI_RS_ADVANCED_SERVER", "hello")].iter());
    new_thrift_session(
        SessionOptions::default()
            .threaded(true)
            .cook_options(CookOptions::default()),
        server_options,
    )
}

fn main() -> Result<()> {
    env_logger::init();
    let _ = simple_default_thrift_session()?;
    let advanced_session = thirft_advanced_setup()?;

    let var = advanced_session.get_server_var::<str>("HAPI_RS_ADVANCED_SERVER")?;
    println!("ADVANCED SERVER VARIABLE: {}", var);

    Ok(())
}
