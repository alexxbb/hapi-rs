use hapi_rs::asset::{AssetLibrary, ParmValue};

mod _utils;

use _utils::*;

#[test]
fn asset_get_count() {
    with_session_asset(Asset::Parameters, |_, lib| {
        assert_eq!(lib.get_asset_count()?, 1);
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_load_from_memory() {
    with_session(|session| {
        let mem = std::fs::read("otls/hapi_geo.hda").unwrap();
        AssetLibrary::from_memory(session.clone(), &mem)?;
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_get_names() {
    with_session_asset(Asset::Parameters, |_, lib| {
        assert!(lib
            .get_asset_names()?
            .contains(&"Object/hapi_parms".to_string()));
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_parameter_tags() {
    with_session_asset(Asset::Parameters, |_, lib| {
        let parms = lib.get_asset_parms("Object/hapi_parms")?;
        let parm = parms.find_parameter("float3").expect("float3 parameter");
        let (tag_name, tag_value) = parm.get_tag(0)?;
        assert_eq!(tag_name, "my_tag");
        assert_eq!(tag_value, "foo");
        Ok(())
    })
    .unwrap()
}

#[test]
fn asset_get_first_name() {
    with_session_asset(Asset::Parameters, |_, lib| {
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
    with_session_asset(Asset::Parameters, |_, lib| {
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
    with_session_asset(Asset::Parameters, |_, lib| {
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
fn asset_create_node() {
    with_session_asset(Asset::Parameters, |_, lib| {
        lib.create_asset_for_node("Object/hapi_parms", None)
            .unwrap();
        lib.create_asset_for_node("Cop2/color", None)?;
        lib.create_asset_for_node("Top/invoke", None)?;
        Ok(())
    })
    .unwrap()
}
