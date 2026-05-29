use hapi_rs::Result;
use hapi_rs::asset::{AssetLibrary, ParmValue};
use hapi_rs::node::NodeType;
use pretty_assertions::assert_eq;
use std::collections::HashSet;

mod utils;

use utils::{HdaFile, with_session, with_session_asset};

#[test]
fn asset_get_count() -> Result<()> {
    with_session_asset(HdaFile::Parameters, |lib| {
        assert_eq!(lib.get_asset_count()?, 1);
        Ok(())
    })
}

#[test]
fn asset_load_from_memory() -> Result<()> {
    with_session(|session| {
        let mem = std::fs::read("../otls/hapi_geo.hda")?;
        AssetLibrary::from_memory(session.clone(), &mem)?;
        Ok(())
    })
}

#[test]
fn asset_get_names() -> Result<()> {
    with_session_asset(HdaFile::Parameters, |lib| {
        assert!(
            lib.get_asset_names()?
                .contains(&"Object/hapi_parms".to_string())
        );
        Ok(())
    })
}

#[test]
fn asset_parameter_tags() -> Result<()> {
    with_session_asset(HdaFile::Parameters, |lib| {
        let parms = lib.get_asset_parms("Object/hapi_parms")?;
        let parm = parms
            .find_parameter("float3")
            .ok_or_else(|| hapi_rs::HapiError::Internal("float3 parameter".into()))?;
        assert_eq!(parm.tag_count(), 2);
        let (tag_name, tag_value) = parm.get_tag(0)?;
        assert_eq!(tag_name, "script_callback_language");
        assert_eq!(tag_value, "python");
        let (tag_name, tag_value) = parm.get_tag(1)?;
        assert_eq!(tag_name, "my_tag");
        assert_eq!(tag_value, "foo");
        Ok(())
    })
}

#[test]
fn asset_get_first_name() -> Result<()> {
    with_session_asset(HdaFile::Parameters, |lib| {
        assert_eq!(
            lib.get_first_name()?,
            Some(String::from("Object/hapi_parms"))
        );
        Ok(())
    })
}

#[test]
fn asset_load_from_file() -> Result<()> {
    with_session(|session| {
        let lib = AssetLibrary::from_file(session.clone(), HdaFile::Parameters.path())?;
        assert_eq!(lib.get_asset_count()?, 1);
        assert_eq!(
            lib.get_first_name()?,
            Some(String::from("Object/hapi_parms"))
        );
        Ok(())
    })
}

#[test]
fn asset_default_parameters() -> Result<()> {
    macro_rules! assert_parm_value {
        ($parm:expr, $variant:ident, $expected:expr) => {
            match $parm {
                ParmValue::$variant(val) => assert_eq!(val, $expected),
                other => {
                    return Err(hapi_rs::HapiError::Internal(format!(
                        "expected {} parameter, got {other:?}",
                        stringify!($variant)
                    )));
                }
            }
        };
    }
    with_session_asset(HdaFile::Parameters, |lib| {
        let all_parms = lib.get_asset_parms("Object/hapi_parms")?;

        let string_parm = all_parms
            .find_parameter("single_string")
            .ok_or_else(|| hapi_rs::HapiError::Internal("single_string parameter".into()))?;
        assert_parm_value!(string_parm.default_value(), String, &["hello"]);
        let float_parm = all_parms
            .find_parameter("float3")
            .ok_or_else(|| hapi_rs::HapiError::Internal("float3 parameter".into()))?;

        let int_button_parm = all_parms
            .find_parameter("button")
            .ok_or_else(|| hapi_rs::HapiError::Internal("button parameter".into()))?;

        assert_parm_value!(int_button_parm.default_value(), Int, &[0]);
        assert!(int_button_parm.menu_items().is_none());

        let toggle_parm = all_parms
            .find_parameter("toggle")
            .ok_or_else(|| hapi_rs::HapiError::Internal("toggle parameter".into()))?;

        assert_parm_value!(toggle_parm.default_value(), Toggle, false);
        let toggle_menu_items = toggle_parm
            .menu_items()
            .expect("toggle parameter should have menu items");
        assert_eq!(toggle_menu_items[0].label()?, "off");
        assert_eq!(toggle_menu_items[0].value()?, "off");

        assert_parm_value!(float_parm.default_value(), Float, &[0.1, 0.2, 0.3]);
        Ok(())
    })
}

#[test]
fn asset_menu_parameters() -> Result<()> {
    with_session_asset(HdaFile::Parameters, |lib| {
        let parms = lib.get_asset_parms("Object/hapi_parms")?;

        let parm = parms
            .find_parameter("string_menu")
            .ok_or_else(|| hapi_rs::HapiError::Internal("string_menu parameter".into()))?;
        let menu_values: Vec<_> = parm
            .menu_items()
            .ok_or_else(|| hapi_rs::HapiError::Internal("Menu items".into()))?
            .iter()
            .map(|p| p.value())
            .collect::<Result<Vec<_>>>()?;
        assert_eq!(menu_values, &["item_1", "item_2", "item_3"]);
        let parm = parms
            .find_parameter("script_menu")
            .ok_or_else(|| hapi_rs::HapiError::Internal("script_menu parameter".into()))?;
        // Script Menus are not evaluated from asset definition, only from a node instance
        assert!(
            parm.menu_items()
                .ok_or_else(|| hapi_rs::HapiError::Internal("Script Items".into()))?
                .is_empty()
        );
        Ok(())
    })
}

#[test]
fn asset_create_node_fully_qualified() -> Result<()> {
    use hapi_rs::HapiError;
    with_session_asset(HdaFile::Parameters, |lib| {
        lib.create_asset_for_node("Object/hapi_parms", None)?;
        lib.create_asset_for_node("Cop2/color", None)?;
        lib.create_asset_for_node("Top/invoke", None)?;
        assert!(matches!(
            lib.create_asset_for_node("foo", None),
            Err(HapiError::Internal(e)) if e.contains("Incomplete node name")
        ));
        Ok(())
    })
}

#[test]
fn asset_try_create_first() -> Result<()> {
    with_session_asset(HdaFile::Parameters, |lib| {
        assert_eq!(
            lib.get_first_name()?,
            Some(String::from("Object/hapi_parms"))
        );
        let node = lib.try_create_first()?;
        assert_eq!(node.info.node_type(), NodeType::Obj);
        Ok(())
    })
}

#[test]
fn asset_parameters_iter() -> Result<()> {
    with_session_asset(HdaFile::Parameters, |lib| {
        assert!(
            lib.get_asset_parms("Object/non-existent-asset-should-fail")
                .is_err()
        );
        let parms = lib.get_asset_parms("Object/hapi_parms")?;
        // test iterator path
        #[allow(unused_variables)]
        let _iter = parms.iter();
        let mut names = HashSet::new();
        for parm in &parms {
            names.insert(parm.name()?);
        }
        for expected in ["single_string", "float3", "string_menu"] {
            assert!(names.contains(expected), "iterator must yield {expected}");
        }
        Ok(())
    })
}
