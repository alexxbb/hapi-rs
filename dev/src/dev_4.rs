pub use hapi_rs::{
    Result,
    ffi,
    geometry::*,
    node::{HoudiniNode, NodeFlags, NodeType},
    parameter::*,
    session::{CookResult, Session, SessionOptions, StatusVerbosity},
    HOUDINI_VERSION,
};
use hapi_rs::ffi::{PartInfo, AttributeInfo, CookOptions};
use hapi_rs::StorageType;
use hapi_rs::session::HapiError;

pub unsafe fn run() -> Result<()> {
    let mut opts = SessionOptions::default();
    opts.cook_opt = CookOptions::default().with_clear_errors_and_warnings(true);
    opts.cleanup = true;
    // opts.unsync = true;
    let mut session = hapi_rs::session::simple_session(Some(&opts))?;
    // let mut session = Session::connect_to_pipe("c:/Temp/hars")?;
    // session.initialize(&opts);
    let otl = std::env::current_dir().unwrap().join("otls/cook_err_fatal.hda");
    let library = session.load_asset_file(otl.to_string_lossy())?;
    let node = library.try_create_first()?;

    if let Err(e) = node.cook(None) {
        println!("Oops: {}\n{}", e, node.cook_result(StatusVerbosity::Errors)?);
    }

    // println!("{}", session.get_cook_result_string(StatusVerbosity::All)?);
    // let err = session.get_cook_result_string(StatusVerbosity::Statusverbosity2)?;
    // println!("Status: {}", err);
    // let geo = node.geometry()?.unwrap();
    // let part = geo.part_info(0)?;
    // let attribs = geo.get_attribute_names(AttributeOwner::Point, &part)?;
    // geo.save_to_file("c:/temp/debug.geo")?;
    // session.save_hip("c:/temp/debug.hip")?;
    Ok(())
}
