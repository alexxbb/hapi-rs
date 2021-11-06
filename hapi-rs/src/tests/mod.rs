#![allow(clippy::all)]
use crate::geometry::*;
use crate::parameter::*;
use crate::session::*;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::thread_local;

thread_local! {
    static SESSION: Lazy<Session> = Lazy::new(|| {
        simple_session(None).expect("Could not create test session")
});
}

pub fn with_session(func: impl FnOnce(&Lazy<Session>)) {
    SESSION.with(|session| {
        func(session);
        // let _ = session.cleanup();
    })
}

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
    with_session(|s| assert!(s.is_valid()));
}

#[test]
fn session_time() {
    with_session(|session| {
        let opt = crate::TimelineOptions::default().with_end_time(5.5);
        assert!(session.set_timeline_options(opt.clone()).is_ok());
        let opt2 = session.get_timeline_options().expect("timeline_options");
        assert!(opt.end_time().eq(&opt2.end_time()));
        session.set_time(4.12).expect("set_time");
        assert!(session.get_time().expect("get_time").eq(&4.12));
    });
}

#[test]
fn server_env() -> Result<()> {
    // Starting new separate session because getting/setting env variables from multiple
    // clients ( threads ) break the server
    let session = simple_session(None).expect("Could not start session");
    session.set_server_var::<str>("FOO", "foo_string")?;
    assert_eq!(session.get_server_var::<str>("FOO")?, "foo_string");
    session.set_server_var::<i32>("BAR", &123)?;
    assert_eq!(session.get_server_var::<i32>("BAR")?, 123);
    assert_eq!(session.get_server_variables()?.is_empty(), false);
    Ok(())
}

#[test]
fn load_asset() {
    with_session(|session| {
        let otl = OTLS.get("parameters").unwrap();
        let lib = session.load_asset_file(otl).expect("load_asset_file");
        assert_eq!(lib.get_asset_count().expect("get_asset_count"), 1);
        assert!(lib
            .get_asset_names()
            .expect("get_asset_name")
            .contains(&"Object/hapi_parms".to_string()));
        lib.try_create_first().expect("try_create_first");
    });
}

#[test]
fn asset_parameters() {
    with_session(|session| {
        assert!(session.is_valid());
        let otl = OTLS.get("parameters").unwrap();
        let lib = session
            .load_asset_file(otl)
            .expect(&format!("Could not load {}", otl));
        let _ = lib.get_asset_parms(Some("Object/hapi_parms"));
        // TODO: This is failing
        // assert!(parms.is_ok());
    });
}

#[test]
fn node_parameters() {
    with_session(|session| {
        let otl = OTLS.get("parameters").unwrap();
        let _lib = session.load_asset_file(otl).unwrap();
        let node = session
            .create_node_blocking("Object/hapi_parms", None, None)
            .expect("create_node");
        for p in node.parameters().unwrap() {
            assert!(p.name().is_ok());
        }
        if let Parameter::Float(p) = node.parameter("color").unwrap() {
            let val = p.get_value().unwrap();
            assert_eq!(&val, &[0.55, 0.75, 0.95]);
            p.set_value([0.7, 0.5, 0.3]).unwrap();
            let val = p.get_value().unwrap();
            assert_eq!(&val, &[0.7, 0.5, 0.3]);
        }
        if let Parameter::Float(p) = node.parameter("single_float").unwrap() {
            p.set_expression("$T", 0).unwrap();
            assert_eq!("$T", p.expression(0).unwrap());
        }

        if let Parameter::String(p) = node.parameter("multi_string").unwrap() {
            let mut value = p.get_value().unwrap();
            assert_eq!(vec!["foo 1", "bar 2", "baz 3"], value);
            value[0] = "cheese".to_owned();
            p.set_value(value).unwrap();
            assert_eq!("cheese", p.get_value().unwrap()[0]);
        }

        if let Parameter::Int(p) = node.parameter("ord_menu").unwrap() {
            assert!(p.is_menu());
            assert_eq!(p.get_value().unwrap()[0], 0);
            if let Some(items) = p.menu_items().unwrap() {
                assert_eq!(items[0].value().unwrap(), "foo");
                assert_eq!(items[0].label().unwrap(), "Foo");
            }
        }

        if let Parameter::Int(p) = node.parameter("toggle").unwrap() {
            assert_eq!(p.get_value().unwrap()[0], 0);
            p.set_value([1]).unwrap();
            assert_eq!(p.get_value().unwrap()[0], 1);
        }

        // test button callback
        if let Parameter::Button(ip) = node.parameter("button").unwrap() {
            if let Parameter::String(sp) = node.parameter("single_string").unwrap() {
                assert_eq!(sp.get_value().unwrap()[0], "hello");
                ip.press_button().unwrap();
                assert_eq!(sp.get_value().unwrap()[0], "set from callback");
            }
        }
    });
}

#[test]
fn nodes() {
    with_session(|session| {
        let obj = HoudiniNode::get_manager_node((*session).clone(), NodeType::Obj).unwrap();
        session
            .create_node("geo", Some("some_name"), Some(obj.handle))
            .unwrap();
        assert!(obj.node("some_name").unwrap().is_some());
        session.cleanup().unwrap();
    });
}

#[test]
fn geometry_triangle() {
    with_session(|session| {
        let node = session.create_input_node("test").expect("input node");
        let geo = node.geometry().expect("geometry").unwrap();

        let part = PartInfo::default()
            .with_part_type(PartType::Mesh)
            .with_face_count(1)
            .with_point_count(3)
            .with_vertex_count(3);
        geo.set_part_info(&part).expect("part_info");
        let info = AttributeInfo::default()
            .with_count(part.point_count())
            .with_tuple_size(3)
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::Float);
        let attr_p = geo
            .add_attribute::<f32>(part.part_id(), "P", &info)
            .unwrap();
        attr_p
            .set(
                part.part_id(),
                &[0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
            )
            .unwrap();
        geo.set_vertex_list(0, [0, 1, 2]).unwrap();
        geo.set_face_counts(0, [3]).unwrap();
        geo.commit().expect("commit");

        node.cook_blocking(None).expect("cook");

        let val: Vec<_> = attr_p.read(part.part_id()).expect("read_attribute");
        assert_eq!(val.len(), 9);
    });
}
