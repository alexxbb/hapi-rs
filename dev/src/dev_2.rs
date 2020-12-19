use hapi_rs::{
    ffi,
    errors::{HapiError, HapiResult, Kind, Result},
    session::{CookResult, Session, SessionOptions, StatusVerbosity},
    node::{HoudiniNode, NodeFlags, NodeType},
    parameter::*,
    HOUDINI_VERSION
};

pub unsafe fn run() -> Result<()> {
    let mut session = Session::new_named_pipe("/tmp/hapi")?;
    // session.cleanup()?;
    let opts = SessionOptions::default().otl_search_paths(&["/Users/alex/sandbox/rust/hapi/otls"]);
    session.initialize(opts);
    let otl = std::env::current_dir()
        .unwrap()
        .join("otls/hapi_parms.hda");
    let library = session.load_asset_file(otl)?;
    let names = library.get_asset_names()?;
    let obj = HoudiniNode::get_manager_node(session.clone(), NodeType::Obj)?;
    let node = session.create_node_blocking(&names[0], None, None)?;
    for p in &node.parameters()? {
        println!("Parm: {}", p.name()?)
    }

    // session.save_hip("/tmp/foo.hip")?;
    Ok(())
}
