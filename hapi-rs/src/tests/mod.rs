#![allow(clippy::all)]
use crate::geometry::*;
use crate::parameter::*;
use crate::session::*;
use once_cell::sync::Lazy;
use std::collections::HashMap;

static SESSION: Lazy<Session> = Lazy::new(|| {
    env_logger::init();
    simple_session(None).expect("Could not create session")
});
pub static OTLS: Lazy<HashMap<&str, String>> = Lazy::new(|| {
    let mut map = HashMap::new();
    let root = format!(
        "{}/otls",
        std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .to_string_lossy()
    );
    map.insert("parameters", format!("{}/hapi_parms.hda", root));
    map
});

#[test]
fn create_and_init() {
    assert!(SESSION.is_valid());
}

#[test]
fn session_time() -> Result<()> {
    let opt = crate::TimelineOptions::default().with_end_time(5.5);
    assert!(SESSION.set_timeline_options(opt.clone()).is_ok());
    let opt2 = SESSION.get_timeline_options()?;
    assert!(opt.end_time().eq(&opt2.end_time()));
    SESSION.set_time(4.12)?;
    assert!(SESSION.get_time()?.eq(&4.12));
    Ok(())
}

#[test]
fn server_env() -> Result<()> {
    SESSION.set_server_var::<str>("FOO", "foo_string")?;
    assert_eq!(SESSION.get_server_var::<str>("FOO")?, "foo_string");
    SESSION.set_server_var::<i32>("BAR", &123)?;
    assert_eq!(SESSION.get_server_var::<i32>("BAR")?, 123);
    assert_eq!(SESSION.get_server_variables()?.is_empty(), false);
    Ok(())
}

#[test]
fn load_asset() -> Result<()> {
    assert!(SESSION.is_valid());
    let otl = OTLS.get("parameters").unwrap();
    let lib = SESSION.load_asset_file(otl)?;
    assert_eq!(lib.get_asset_count()?, 1);
    assert!(lib
        .get_asset_names()?
        .contains(&"Object/hapi_parms".to_string()));
    lib.try_create_first()?;
    Ok(())
}

/// This crashes HARS do to sesi bug
#[allow(unused)]
fn asset_parameters() {
    assert!(SESSION.is_valid());
    let otl = OTLS.get("parameters").unwrap();
    let lib = SESSION
        .load_asset_file(otl)
        .expect(&format!("Could not load {}", otl));
    let parms = lib.get_asset_parms("Object/hapi_parms");
    assert!(parms.is_ok());
}

#[test]
fn node_parameters() -> Result<()> {
    let otl = OTLS.get("parameters").unwrap();
    let _lib = SESSION.load_asset_file(otl)?;
    let node = SESSION.create_node_blocking("Object/hapi_parms", None, None)?;
    for p in node.parameters()? {
        assert!(p.name().is_ok());
    }
    if let Parameter::Float(p) = node.parameter("color")? {
        let val = p.get_value()?;
        assert_eq!(&val, &[0.55, 0.75, 0.95]);
        p.set_value([0.7, 0.5, 0.3])?;
        let val = p.get_value()?;
        assert_eq!(&val, &[0.7, 0.5, 0.3]);
    }
    if let Parameter::String(p) = node.parameter("single_float")? {
        p.set_expression("$T", 0)?;
        assert_eq!("$T", p.expression(0)?);
    }

    if let Parameter::String(p) = node.parameter("multi_string")? {
        let mut value = p.get_value()?;
        assert_eq!(vec!["foo 1", "bar 2", "baz 3"], value);
        value[0] = "cheese".to_owned();
        p.set_value(value)?;
        assert_eq!("cheese", p.get_value()?[0]);
    }

    if let Parameter::Int(p) = node.parameter("ord_menu")? {
        assert!(p.is_menu());
        assert_eq!(p.get_value()?[0], 0);
        let items = p.menu_items().unwrap()?;
        assert_eq!(items[0].value()?, "foo");
        assert_eq!(items[0].label()?, "Foo");
    }

    if let Parameter::Int(p) = node.parameter("toggle")? {
        assert_eq!(p.get_value()?[0], 0);
        p.set_value([1])?;
        assert_eq!(p.get_value()?[0], 1);
    }

    // test button callback
    if let Parameter::Button(ip) = node.parameter("button")? {
        if let Parameter::String(sp) = node.parameter("single_string")? {
            assert_eq!(sp.get_value()?[0], "hello");
            ip.press_button()?;
            assert_eq!(sp.get_value()?[0], "set from callback");
        }
    }
    Ok(())
}

#[test]
fn nodes() -> Result<()> {
    let obj = HoudiniNode::get_manager_node(SESSION.clone(), NodeType::Obj)?;
    SESSION.create_node("geo", Some("some_name"), Some(obj.handle))?;
    assert!(obj.node("some_name")?.is_some());
    Ok(())
}


#[test]
fn geometry_triangle() -> Result<()> {
    let node = SESSION.create_input_node("test")?;
    let geo = node.geometry()?.unwrap();

    let part = PartInfo::default()
        .with_part_type(PartType::Mesh)
        .with_face_count(1)
        .with_point_count(3)
        .with_vertex_count(3);
    geo.set_part_info(&part)?;
    let info = AttributeInfo::default()
        .with_count(part.point_count())
        .with_tuple_size(3)
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Float);
    let attr_p = geo.add_attribute::<f32>(part.part_id(), "P", &info)?;
    attr_p.set(
        part.part_id(),
        &[0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
    )?;
    geo.set_vertex_list(0, [0, 1, 2])?;
    geo.set_face_counts(0, [3])?;
    geo.commit()?;

    node.cook_blocking(None)?;

    let val: Vec<_> = attr_p.read(part.part_id())?;
    assert_eq!(val.len(), 9);
    Ok(())
}
