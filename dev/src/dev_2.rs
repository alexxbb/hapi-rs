use hapi_rs::{
    ffi,
    errors::{HapiError, HapiResult, Kind, Result},
    session::{CookResult, Session, SessionOptions, StatusVerbosity},
    node::{HoudiniNode, NodeFlags, NodeType},
    HOUDINI_VERSION
};

pub unsafe fn run() -> Result<()> {
    let mut session = Session::new_named_pipe("/tmp/hapi")?;
    // session.cleanup()?;
    let opts = SessionOptions::default().otl_search_paths(&["/Users/alex/sandbox/rust/hapi/otls"]);
    if let Err(e) = session.initialize(opts) {
        if !matches!(e.kind, Kind::Hapi(HapiResult::AlreadyInitialized)) {
            return Err(e);
        }
    }
    let otl = std::env::current_dir()
        .unwrap()
        .join("otls/FourShapes.hda");
    let library = session.load_asset_file(otl)?;
    let names = library.get_asset_names()?;
    let obj = HoudiniNode::get_manager_node(session.clone(), NodeType::Obj)?;
    let node = session.create_node_blocking(&names[0], None, None)?;
    match node.cook_blocking(None)? {
        CookResult::Succeeded => println!("Cooking Done!"),
        CookResult::Warnings => {
            let w = session.get_cook_status(StatusVerbosity::Warnings)?;
            println!("Warnings: {}", w);
        }
        CookResult::Errored(err) => {
            println!("Errors: {}", err);
        }
    }
    node.cook_blocking(None)?;
    let cc = node.cook_count(NodeType::Any, NodeFlags::Any)?;
    println!("CC: {}", cc);
    let info = node.info()?;
    let cc = node.cook_count(NodeType::Any, NodeFlags::Any)?;
    println!("Manager: {:?}", HoudiniNode::get_manager_node(session.clone(), NodeType::Obj)?);
    let children = node.get_children(NodeType::Any, NodeFlags::Any, true)?;
    println!("Parent: {}", node.parent_node()?.info(&session)?.name()?);
    for ch in children {
        let info = ch.info(&session)?;
        // println!("{}", info.name()?)
    }
    assert!(node.parameter("foo_bar").is_err());
    let scale = node.parameter("scale")?;
    Ok(())
}
