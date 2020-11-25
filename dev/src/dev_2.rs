use hapi_rs::{
    errors::{Result, HapiResult, HapiError, Kind},
    session::{Session, SessionOptions},
    StatusVerbosity
};
use smol;

pub unsafe fn run() -> Result<()> {
    let mut session = Session::new_named_pipe("/tmp/hapi")?;
    // session.cleanup()?;
    let opts = SessionOptions::default().otl_search_paths(&["/Users/alex/sandbox/rust/hapi/otls"]);
    if let Err(e) = session.initialize(opts) {
        if !matches!(e.kind, Kind::Hapi(HapiResult::AlreadyInitialized)) {
            return Err(e)
        }
    }
    let library = session.load_asset_file("/Users/alex/sandbox/rust/hapi/otls/spaceship.otl")?;
    let names = library.get_asset_names()?;
    let node = session.create_node_blocking(&names[0], None, None)?;
    println!("Need to cook: {}", session.cooking_total_count()?);
    println!("Already cooked: {}", session.cooking_current_count()?);
    // node.cook_blocking(None)?;
    // println!("{}", session.last_cook_error(None)?);
    // session.save_hip("/tmp/foo.hip")?;
    Ok(())
}
