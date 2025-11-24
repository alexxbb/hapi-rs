use std::time::Duration;

use hapi_rs::Result;
use hapi_rs::node::CookOptions;
use hapi_rs::server::*;
use hapi_rs::session::{LicenseType, Session, SessionOptions, new_thrift_session};

fn simple_default_thrift_session() -> Result<Session> {
    new_thrift_session(
        SessionOptions::default(),
        ServerOptions::shared_memory_with_defaults(),
    )
}

fn thirft_advanced_setup() -> Result<Session> {
    let server_options = ServerOptions::default()
        .with_thrift_transport(ThriftTransport::SharedMemory(
            ThriftSharedMemoryTransportBuilder::default()
                .with_memory_name("hapi-rs-advanced-server")
                .with_buffer_size(1000)
                .with_buffer_type(ThriftSharedMemoryBufferType::Buffer)
                .build(),
        ))
        .with_connection_timeout(Some(Duration::from_secs(10)))
        .with_env_variables(vec![("HAPI_RS_ADVANCED_SERVER", "hello")].iter())
        .with_license_preference(LicensePreference::HoudiniEngineOnly);
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
    println!("Simple default thrift session created");

    let session = thirft_advanced_setup()?;
    session.create_node("Object/null")?.cook_blocking()?;
    assert_eq!(session.get_license_type()?, LicenseType::HoudiniEngine);
    println!(
        "Advanced thrift session created with license type: {:?}",
        LicenseType::HoudiniEngine
    );

    Ok(())
}
