use hapi_rs::{
    errors::{HapiError, HapiResult, Kind, Result},
    session::{CookResult, Session, SessionOptions},
    NodeFlags, NodeType, StatusVerbosity, HOUDINI_VERSION
};
use hapi_rs::node::HoudiniNode;

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
        .join("otls/nurbs_curve.hda");
    let library = session.load_asset_file(otl)?;
    let names = library.get_asset_names()?;
    let node = session.create_node_blocking(&names[0], None, None)?;
    match node.cook_blocking(None)? {
        CookResult::Succeeded => println!("Cooking Done!"),
        CookResult::Warnings => {
            let w = session.get_cook_status(StatusVerbosity::VerbosityWarnings)?;
            println!("Warnings: {}", w);
        }
        CookResult::Errored(err) => {
            println!("Errors: {}", err);
        }
    }
    node.cook_blocking(None)?;
    let cc = node.cook_count(-1, -1)?;
    println!("CC: {}", cc);
    let info = node.info()?;
    println!("{:#?}", info);
    let cc = node.cook_count(-1, -1)?;
    let geo = session.create_node_blocking("Object/geo", None, None)?;
    println!("Manager: {:?}", HoudiniNode::get_manager_node(session.clone(), NodeType::Obj)?);
    Ok(())
}
