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
    // let mut session = hapi_rs::session::simple_session(Some(&opts))?;
    let mut session = Session::connect_to_pipe("c:/Temp/hapi")?;
    session.initialize(&opts);
    let otl = std::env::current_dir().unwrap().join("otls/hapi_parms.hda");
    let library = session.load_asset_file(otl.to_string_lossy())?;

    let parms = library.get_asset_parms(&library.get_asset_names()?[0])?;
    for p in &parms {
        println!("{:?}", p.default_values());
    }


    Ok(())
}
