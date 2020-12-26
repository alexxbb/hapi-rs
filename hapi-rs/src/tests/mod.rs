use once_cell::sync::Lazy;
use crate::session::*;
use crate::parameter::*;
use std::collections::HashMap;

static SESSION: Lazy<Session> = Lazy::new(|| {
    use env_logger;
    env_logger::init();
    let tmp = std::env::var("TMP").or_else(|_| std::env::var("TEMP")).expect("Could not get TEMP dir");
    let pipe = format!("{}/hapi_test_pipe", tmp);
    Session::start_engine_server(&pipe, true, 2000.0).expect("Could not start test session");
    let mut ses = Session::connect_to_server(&pipe).expect("Could not create thrift session");
    ses.initialize(SessionOptions::default());
    ses
});
pub static OTLS: Lazy<HashMap<&str, String>> = Lazy::new(|| {
    let mut map = HashMap::new();
    let root = format!("{}/otls", std::env::current_dir().unwrap().parent().unwrap().to_string_lossy());
    map.insert("parameters", format!("{}/hapi_parms.hda", root));
    map
});

#[test]
fn create_and_init() {
    assert!(SESSION.is_valid());
}

// #[test]
fn load_asset() {
    assert!(SESSION.is_valid());
    let otl = OTLS.get("parameters").unwrap();
    let lib = SESSION.load_asset_file(otl).expect(&format!(" Could not load {}", otl));
    assert_eq!(lib.get_asset_count().unwrap(), 1);
    assert!(lib.get_asset_names().unwrap().contains(&"Object/hapi_parms".to_string()));
}

#[test]
fn asset_parameters() {
    assert!(SESSION.is_valid());
    let otl = OTLS.get("parameters").unwrap();
    let lib = SESSION.load_asset_file(otl).expect(&format!("Could not load {}", otl));
    let parms = lib.get_asset_parms("Object/hapi_parms");
    assert!(parms.is_ok());
}

// #[test]
fn node_parameters() {
    // if let Parameter::Float(mut p) = node.parameter("color").unwrap() {
    //     let val = p.get_value().unwrap();
    //     assert_eq!(&val, &[0.55, 0.75, 0.95]);
    //     p.set_value([0.7, 0.5, 0.3]).unwrap();
    //     let val = p.get_value().unwrap();
    //     assert_eq!(&val, &[0.7, 0.5, 0.3]);
    // }
}
