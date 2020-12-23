#![allow(unused)]
use log;
#[macro_use]
mod macros;
#[macro_use]
pub mod errors;
mod asset;
mod auto;
pub mod node;
pub mod session;
mod stringhandle;
mod attribute;
pub mod parameter;

pub use auto::bindings as ffi;
pub use errors::Result;
pub use stringhandle::get_string;
#[derive(Debug)]
pub struct _HoudiniVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    pub patch: u32,
}

#[derive(Debug)]
pub struct _EngineVersion {
    pub major: u32,
    pub minor: u32,
    pub api: u32,
}

pub const HOUDINI_VERSION: _HoudiniVersion = _HoudiniVersion {
    major: ffi::HAPI_VERSION_HOUDINI_MAJOR,
    minor: ffi::HAPI_VERSION_HOUDINI_MINOR,
    build: ffi::HAPI_VERSION_HOUDINI_BUILD,
    patch: ffi::HAPI_VERSION_HOUDINI_PATCH,
};

pub const ENGINE_VERSION: _EngineVersion = _EngineVersion {
    major: ffi::HAPI_VERSION_HOUDINI_ENGINE_MAJOR,
    minor: ffi::HAPI_VERSION_HOUDINI_ENGINE_MINOR,
    api: ffi::HAPI_VERSION_HOUDINI_ENGINE_API,
};

#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;
    use crate::session::*;
    use std::collections::HashMap;

    static OTLS: Lazy<HashMap<&str, String>> = Lazy::new(||{
       let mut map = HashMap::new();
        let root = format!("{}/otls", std::env::current_dir().unwrap().parent().unwrap().to_string_lossy());
        map.insert("parameters", format!("{}/hapi_parms.hda", root));
        map
    });

    static SESSION: Lazy<Session> = Lazy::new(||{
        let tmp = std::env::var("TMP").or_else(|_|std::env::var("TEMP")).expect("Could not get TEMP dir");
        let pipe = format!("{}/hapi_test_pipe", tmp);
        Session::start_named_pipe_server(&pipe, true, 2000.0).expect("Could not start test session");
        let mut ses = Session::new_named_pipe(&pipe).expect("Could not create thrift session");
        ses.initialize(SessionOptions::default());
        ses

    });

    #[test]
    fn create_and_init() {
        assert!(SESSION.is_valid().unwrap());
    }

    #[test]
    fn load_asset() {
        let otl = OTLS.get("parameters").unwrap();
        let lib = SESSION.load_asset_file(otl).expect(&format!("Could not load {}", otl));
        assert_eq!(lib.get_asset_count().unwrap(), 1);
    }
}
