use hapi_rs::asset::{AssetLibrary, ParmValue};
use hapi_rs::node::NodeType;
use std::collections::HashSet;

mod utils;

use utils::{HdaFile, with_session, with_session_asset};

#[test]
fn asset_get_count() {
    with_session_asset(HdaFile::Parameters, |lib| {
        assert_eq!(lib.get_asset_count()?, 1);
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_load_from_memory() {
    with_session(|session| {
        let mem = std::fs::read("../otls/hapi_geo.hda").unwrap();
        AssetLibrary::from_memory(session.clone(), &mem)?;
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_get_names() {
    with_session_asset(HdaFile::Parameters, |lib| {
        assert!(
            lib.get_asset_names()?
                .contains(&"Object/hapi_parms".to_string())
        );
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_parameter_tags() {
    with_session_asset(HdaFile::Parameters, |lib| {
        let parms = lib.get_asset_parms("Object/hapi_parms")?;
        let parm = parms.find_parameter("float3").expect("float3 parameter");
        assert_eq!(parm.tag_count(), 2);
        let (tag_name, tag_value) = parm.get_tag(0)?;
        assert_eq!(tag_name, "script_callback_language");
        assert_eq!(tag_value, "python");
        let (tag_name, tag_value) = parm.get_tag(1)?;
        assert_eq!(tag_name, "my_tag");
        assert_eq!(tag_value, "foo");
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_get_first_name() {
    with_session_asset(HdaFile::Parameters, |lib| {
        assert_eq!(
            lib.get_first_name()?,
            Some(String::from("Object/hapi_parms"))
        );
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_load_from_file() {
    with_session(|session| {
        let lib = AssetLibrary::from_file(session.clone(), HdaFile::Parameters.path())?;
        assert_eq!(lib.get_asset_count()?, 1);
        assert_eq!(
            lib.get_first_name()?,
            Some(String::from("Object/hapi_parms"))
        );
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_default_parameters() {
    with_session_asset(HdaFile::Parameters, |lib| {
        let parms = lib.get_asset_parms("Object/hapi_parms")?;

        let parm = parms.find_parameter("single_string").unwrap();
        if let ParmValue::String([val]) = parm.default_value() {
            assert_eq!(val, "hello");
        }
        let parm = parms.find_parameter("float3").expect("parm");
        if let ParmValue::Float(val) = parm.default_value() {
            assert_eq!(val, &[0.1, 0.2, 0.3]);
        } else {
            panic!("parm is not a float3");
        }
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_menu_parameters() {
    with_session_asset(HdaFile::Parameters, |lib| {
        let parms = lib.get_asset_parms("Object/hapi_parms")?;

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
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_create_node_fully_qualified() {
    use hapi_rs::HapiError;
    with_session_asset(HdaFile::Parameters, |lib| {
        lib.create_asset_for_node("Object/hapi_parms", None)
            .unwrap();
        lib.create_asset_for_node("Cop2/color", None)?;
        lib.create_asset_for_node("Top/invoke", None)?;
        assert!(matches!(
            lib.create_asset_for_node("foo", None),
            Err(HapiError::Internal(e)) if e.contains("Incomplete node name")
        ));
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_try_create_first() {
    with_session_asset(HdaFile::Parameters, |lib| {
        assert_eq!(
            lib.get_first_name()?,
            Some(String::from("Object/hapi_parms"))
        );
        let node = lib.try_create_first()?;
        assert_eq!(node.info.node_type(), NodeType::Obj);
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_parameters_iter() {
    with_session_asset(HdaFile::Parameters, |lib| {
        let parms = lib.get_asset_parms("Object/hapi_parms")?;
        let mut names = HashSet::new();
        for parm in &parms {
            names.insert(parm.name().expect("parameter name"));
        }
        for expected in ["single_string", "float3", "string_menu"] {
            assert!(names.contains(expected), "iterator must yield {expected}");
        }
        Ok(())
    })
    .unwrap()
}
