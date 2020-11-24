use hapi_rs::{
    errors::{Result, HapiResult, HapiError, Kind},
    session::{Session, SessionOptions},
    StatusVerbosity
};
use smol;

pub unsafe fn run() -> Result<()> {
    let mut session = Session::new_named_pipe("/tmp/hapi")?;
    let opts = SessionOptions::default().otl_search_paths(&["/Users/alex/sandbox/rust/hapi/otls"]);
    if let Err(e) = session.initialize(opts) {
        if !matches!(e.kind, Kind::Hapi(HapiResult::AlreadyInitialized)) {
            return Err(e)
        }
    }
    let library = session.load_asset_file("/Users/alex/sandbox/rust/hapi/otls/spaceship.otl")?;
    let names = library.get_asset_names()?;
    let node = session.create_node_blocking(&names[0], None, None)?;
    node.cook_blocking()?;
    println!("{}", session.last_cook_error(None)?);
    // session.save_hip("/tmp/foo.hip")?;
    Ok(())
}
