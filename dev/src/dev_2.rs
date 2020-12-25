use hapi_rs::{
    errors::{HapiError, HapiResult, Kind, Result},
    session::{CookResult, Session, SessionOptions, StatusVerbosity},
    node::{HoudiniNode, NodeFlags, NodeType},
    parameter::*,
    ffi,
    HOUDINI_VERSION
};

pub unsafe fn run() -> Result<()> {
    let mut session = Session::new_named_pipe("/tmp/hapi")?;
    // session.cleanup()?;
    let mut opts = SessionOptions::default();
    opts.set_otl_search_paths(&["/Users/alex/sandbox/rust/hapi/otls"]);
    session.initialize(opts);
    let otl = std::env::current_dir()
        .unwrap()
        .join("otls/spaceship.otl");
    let library = session.load_asset_file(otl.to_string_lossy())?;
    let names = library.get_asset_names()?;
    // let obj = HoudiniNode::get_manager_node(session.clone(), NodeType::Obj)?;
    // let node = session.create_node_blocking(&names[0], None, None)?;

    match library.get_asset_parms(&names[0]) {
        Ok(p) => {}
        Err(e) => {eprintln!("{}", e)}
    }

    Ok(())
}
