use once_cell::sync::Lazy;
use std::sync::Mutex;

use hapi_rs::{
    asset::{AssetLibrary, ParmValue},
    session::{quick_session, Session},
};

static SESSION: Lazy<Mutex<Session>> = Lazy::new(|| {
    // Put session into Mutex for this test since there seem to be a race condition
    // IN Houdini which messes up string access, despite HAPI claims to be thread-safe.
    // This crate used to have an internal reentrant lock for such cases, but it was removed.
    env_logger::init();
    let session = quick_session(None).expect("Could not create test session");
    session
        .load_asset_file("otls/hapi_geo.hda")
        .expect("load asset");
    session
        .load_asset_file("otls/hapi_vol.hda")
        .expect("load asset");
    session
        .load_asset_file("otls/hapi_parms.hda")
        .expect("load asset");
    session
        .load_asset_file("otls/sesi/SideFX_spaceship.hda")
        .expect("load asset");
    Mutex::new(session)
});

static LIB: Lazy<AssetLibrary> = Lazy::new(|| {
    SESSION
        .lock()
        .unwrap()
        .load_asset_file("otls/hapi_parms.hda")
        .expect("load_asset_file")
});

#[test]
fn asset_get_count() {
    assert_eq!(LIB.get_asset_count().expect("get_asset_count"), 1);
}

#[test]
fn asset_load_from_memory() {
    let mem = std::fs::read("otls/hapi_geo.hda").unwrap();
    AssetLibrary::from_memory(SESSION.lock().unwrap().clone(), &mem).unwrap();
}

#[test]
fn asset_get_names() {
    assert!(LIB
        .get_asset_names()
        .expect("get_asset_names")
        .contains(&"Object/hapi_parms".to_string()));
}

#[test]
fn asset_get_first_name() {
    assert_eq!(
        LIB.get_first_name().unwrap(),
        Some(String::from("Object/hapi_parms"))
    );
}

#[test]
fn asset_default_parameters() {
    let parms = LIB.get_asset_parms("Object/hapi_parms").unwrap();

    let parm = parms.find_parameter("single_string").expect("parm");
    if let ParmValue::String([val]) = parm.default_value() {
        assert_eq!(val, "hello");
    }
    let parm = parms.find_parameter("float3").expect("parm");
    if let ParmValue::Float(val) = parm.default_value() {
        assert_eq!(val, &[0.1, 0.2, 0.3]);
    } else {
        panic!("parm is not a float3");
    }
}

#[test]
fn asset_menu_parameters() {
    let parms = LIB.get_asset_parms("Object/hapi_parms").unwrap();

    let parm = parms.find_parameter("string_menu").expect("parm");
    let menu_values: Vec<_> = parm
        .menu_items()
        .expect("Menu items")
        .iter()
        .map(|p| p.value().unwrap())
        .collect();
    assert_eq!(menu_values, &["item_1", "item_2", "item_3"]);
    let parm = parms.find_parameter("script_menu").expect("parm");
    // Script Menus are not evaluated from asset definition, only from a node instance
    assert!(parm.menu_items().expect("Script Items").is_empty());
}

#[test]
fn asset_create_node() {
    LIB.create_asset_for_node("Object/hapi_parms", None)
        .unwrap();
    LIB.create_asset_for_node("Cop2/color", None).unwrap();
    LIB.create_asset_for_node("Top/invoke", None).unwrap();
}
