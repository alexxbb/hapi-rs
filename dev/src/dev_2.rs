pub use hapi_rs::{
    errors::{HapiError, HapiResult, Kind, Result},
    session::{CookResult, Session, SessionOptions, StatusVerbosity},
    node::{HoudiniNode, NodeFlags, NodeType},
    parameter::*,
    ffi,
    HOUDINI_VERSION
};

pub unsafe fn run() -> Result<()> {
    let mut session = Session::connect_to_server("/tmp/hapi")?;
    // let mut session = Session::new_in_process()?;
    // session.cleanup()?;
    let mut opts = SessionOptions::default();
    // opts.set_otl_search_paths(&["/Users/alex/CLionProjects/hapi-rs/otls"]);
    session.initialize(&opts);
    let otl = std::env::current_dir()
        .unwrap()
        .join("otls/spaceship.otl");
    let library = session.load_asset_file(otl.to_string_lossy())?;
    let names = library.get_asset_names()?;
    println!("{:?}", &names);
    // let obj = HoudiniNode::get_manager_node(session.clone(), NodeType::Obj)?;
    // let node = session.create_node_blocking(&names[0], None, None)?;
    // let node = session.create_node_blocking("Object/hapi_parms", None, None)?;

    Ok(())
}
