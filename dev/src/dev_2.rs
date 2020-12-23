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
    let mut opts = SessionOptions::default();
    opts.set_otl_search_paths(&["/Users/alex/sandbox/rust/hapi/otls"]);
    session.initialize(opts);
    let otl = std::env::current_dir()
        .unwrap()
        .join("otls/hapi_parms.hda");
    let library = session.load_asset_file(otl.to_string_lossy())?;
    let names = library.get_asset_names()?;
    let obj = HoudiniNode::get_manager_node(session.clone(), NodeType::Obj)?;
    let node = session.create_node_blocking(&names[0], None, None)?;
    let cam = session.create_node_blocking("cam", None, Some(obj.handle))?;
    let i = cam.asset_info()?;
    for p in &node.parameters()? {
    }

    if let Parameter::Int(mut p) = node.parameter("ord_menu")? {
        if let Some(items) = p.menu_items() {
        }
    }
    if let Parameter::Float(mut p) = node.parameter("single_float")? {
        p.set_expression("$T", 0)?;
        let v = p.expression(0)?;
        dbg!(v);
    }
    let info = node.asset_info()?;
    // session.save_hip("/tmp/foo.hip")?;
    Ok(())
}
