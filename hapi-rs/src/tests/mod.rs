use once_cell::sync::Lazy;
use crate::session::*;
use crate::parameter::*;
use std::collections::HashMap;

static SESSION: Lazy<Session> = Lazy::new(|| {
    use env_logger;
    env_logger::init();
    let tmp = std::env::var("TMP").or_else(|_| std::env::var("TEMP")).expect("Could not get TEMP dir");
    let pipe = format!("{}/hapi_test_pipe", tmp);
    start_engine_server(&pipe, true, 2000.0).expect("Could not start test session");
    let mut ses = Session::connect_to_server(&pipe).expect("Could not create thrift session");
    ses.initialize(&SessionOptions::default());
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

#[test]
fn load_asset() ->Result<()> {
    assert!(SESSION.is_valid());
    let otl = OTLS.get("parameters").unwrap();
    let lib = SESSION.load_asset_file(otl)?;
    assert_eq!(lib.get_asset_count()?, 1);
    assert!(lib.get_asset_names()?.contains(&"Object/hapi_parms".to_string()));
    Ok(())
}

// #[test]
// #[should_panic]
fn asset_parameters() {
    assert!(SESSION.is_valid());
    let otl = OTLS.get("parameters").unwrap();
    let lib = SESSION.load_asset_file(otl).expect(&format!("Could not load {}", otl));
    let parms = lib.get_asset_parms("Object/hapi_parms");
    assert!(parms.is_ok());
}

#[test]
fn node_parameters() -> Result<()> {
    let otl = OTLS.get("parameters").expect("no such key");
    let lib = SESSION.load_asset_file(otl)?;
    let node = SESSION.create_node_blocking("Object/hapi_parms", None, None)?;
    for p in node.parameters()? {
        assert!(p.name().is_ok());
    }
    if let Parameter::Float(mut p) = node.parameter("color")? {
        let val = p.get_value()?;
        assert_eq!(&val, &[0.55, 0.75, 0.95]);
        p.set_value([0.7, 0.5, 0.3])?;
        let val = p.get_value()?;
        assert_eq!(&val, &[0.7, 0.5, 0.3]);
    }
    if let Parameter::String(mut p) = node.parameter("single_float")? {
        p.set_expression("$T", 0)?;
        assert_eq!("$T", p.expression(0)?);
    }

    if let Parameter::String(mut p) = node.parameter("multi_string")? {
        let mut value = p.get_value()?;
        assert_eq!(vec!["foo 1", "bar 2", "baz 3"], value);
        value[0] = "cheese".to_owned();
        p.set_value(value);
        assert_eq!("cheese", p.get_value()?[0]);
    }

    if let Parameter::Int(mut p) = node.parameter("ord_menu")? {
        assert!(p.is_menu());
        assert_eq!(p.get_value()?[0], 0);
        let items = p.menu_items().unwrap()?;
        assert_eq!(items[0].value()?, "foo");
        assert_eq!(items[0].label()?, "Foo");
    }

    if let Parameter::Int(mut p) = node.parameter("toggle")? {
        assert_eq!(p.get_value()?[0], 0);
        p.set_value([1]);
        assert_eq!(p.get_value()?[0], 1);
    }

    // test button callback
    if let Parameter::Int(mut ip) = node.parameter("button")? {
        if let Parameter::String(mut sp) = node.parameter("single_string")? {
            assert_eq!(sp.get_value()?[0], "hello");
            ip.set_value([1]);
            assert_eq!(sp.get_value()?[0], "set from callback");

        }
    }
    Ok(())
}
