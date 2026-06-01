use hapi_rs::geometry::CookOptions;
use hapi_rs::session::CookResult;
use hapi_rs::{
    Result,
    node::{
        HoudiniNode, KeyFrame, ManagerType, NodeFlags, NodeHandle, NodeType, PresetType, RSTOrder,
        StatusVerbosity, ToNodeFlagsBits, ToNodeTypeBits, TransformComponent, TransformEuler,
    },
    parameter::Parameter,
    raw::ErrorCode,
};
use pretty_assertions::assert_eq;
use std::str::FromStr;

mod utils;
use utils::{HdaFile, with_session, with_session_asset};

#[test]
fn node_create() -> Result<()> {
    with_session(|session| {
        session.load_asset_file(HdaFile::Spaceship.path())?;
        let node = session.create_node("Object/spaceship")?;
        assert_eq!(node.cook_count(NodeType::None, NodeFlags::None, true)?, 0);
        node.cook()?; // in threaded mode always successful
        assert_eq!(node.cook_count(NodeType::None, NodeFlags::None, true)?, 1);
        assert!(matches!(session.cook()?, CookResult::Succeeded));

        assert_eq!(format!("{}", NodeType::Sop), "Sop");
        assert_eq!(format!("{}", NodeType::Rop), "Rop");
        assert_eq!(format!("{}", NodeType::Top), "Unknown");
        let _ = (NodeType::Sop | NodeType::Obj).to_bits();
        let _ = (NodeFlags::Display | NodeFlags::Render).to_bits();
        let _ = NodeType::Sop.to_bits() | NodeType::Top;
        let _ = NodeFlags::Display.to_bits() | NodeFlags::Render;

        assert!(!node.name()?.is_empty());
        let _ = node.get_info()?;
        assert!(format!("{node:?}").contains("HoudiniNode"));
        assert!(matches!(
            node.cook_with_options(&CookOptions::default(), false)?,
            CookResult::Succeeded
        ));
        Ok(())
    })
}

#[test]
fn node_set_node_display_flag() -> Result<()> {
    with_session(|session| {
        let sop = session.create_node("Object/geo")?;
        let sphere = session.node_builder("sphere").with_parent(&sop).create()?;
        let cube = session.node_builder("box").with_parent(&sop).create()?;
        sphere.cook()?;
        cube.cook()?;
        cube.set_display_flag(true)?;
        session.cook()?;
        assert!(
            !sphere
                .geometry()?
                .ok_or_else(|| hapi_rs::HapiError::Internal("sphere geometry".into()))?
                .geo_info()?
                .is_display_geo()
        );
        sphere.set_display_flag(true)?;
        assert!(
            !cube
                .geometry()?
                .ok_or_else(|| hapi_rs::HapiError::Internal("cube geometry".into()))?
                .geo_info()?
                .is_display_geo()
        );
        Ok(())
    })
}

#[test]
fn node_inputs_and_outputs() -> Result<()> {
    with_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        let geo = node
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("geometry".into()))?;
        let mut input = geo.node.input_node(0)?;
        while let Some(ref n) = input {
            assert!(n.is_valid()?);
            input = n.input_node(0)?;
        }
        let outputs = geo.node.output_connected_nodes(0, false)?;
        assert!(outputs.is_empty());
        let n = node
            .get_child_by_path("geo/point_attr")?
            .ok_or_else(|| hapi_rs::HapiError::Internal("child node".into()))?;
        assert_eq!(n.get_input_name(0)?, "Geometry to Process with Wrangle");
        Ok(())
    })
}

#[test]
fn node_search() -> Result<()> {
    with_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let asset = session.create_node("Object/hapi_geo")?;
        asset.cook_blocking()?;
        let nope = asset.get_child_by_path("bla")?;
        assert!(nope.is_none());
        asset
            .get_child_by_path("geo/add_color")?
            .ok_or_else(|| hapi_rs::HapiError::Internal("color node".into()))?;
        asset
            .find_child_node("add_color", true)?
            .ok_or_else(|| hapi_rs::HapiError::Internal("color node".into()))?;
        let children = asset.get_children_by_type(NodeType::Sop, NodeFlags::Display, true)?;
        assert_ne!(children.len(), 0);
        assert!(!asset.get_children()?.is_empty());

        let parent = asset
            .parent_node()
            .ok_or_else(|| hapi_rs::HapiError::Internal("parent node".into()))?;
        let subnet = asset
            .get_child_by_path("geo")?
            .ok_or_else(|| hapi_rs::HapiError::Internal("geo subnet".into()))?;
        assert!(!subnet.path_relative(Some(parent))?.is_empty());
        Ok(())
    })
}

#[test]
fn node_transform() -> Result<()> {
    with_session(|session| {
        let obj = session.create_node("Object/null")?;
        let t = obj.get_transform(None, None)?;
        assert_eq!(t.position(), [0.0, 0.0, 0.0]);
        assert_eq!(t.scale(), [1.0, 1.0, 1.0]);
        assert_eq!(t.rst_order(), RSTOrder::Default);
        obj.set_transform(
            &TransformEuler::default()
                .with_position([0.0, 1.0, 0.0])
                .with_rotation([45.0, 0.0, 0.0]),
        )?;
        obj.cook_blocking()?;
        assert!(obj.get_object_info()?.has_transform_changed());
        let t = obj.get_transform(None, None)?;
        assert_eq!(t.position(), [0.0, 1.0, 0.0]);
        session.get_composed_object_transform(
            obj.parent_node()
                .ok_or_else(|| hapi_rs::HapiError::Internal("parent node".into()))?,
            RSTOrder::Default,
        )?;
        Ok(())
    })
}

#[test]
fn node_save_and_load() -> Result<()> {
    with_session(|session| {
        let cam = session.create_node("Object/cam")?;
        let tmp_file = tempfile::NamedTempFile::new()?;
        cam.save_to_file(tmp_file.path().to_string_lossy().as_ref())?;
        let new = HoudiniNode::load_from_file(&session, None, "loaded_cam", true, tmp_file.path())?;
        cam.delete()?;
        new.delete()?;
        assert!(tmp_file.path().exists());
        Ok(())
    })
}

#[test]
fn node_number_of_geo_outputs() -> Result<()> {
    with_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        assert_eq!(node.number_of_geo_outputs()?, 2);
        let infos = node.geometry_output_nodes()?;
        assert_eq!(infos.len(), 2);
        assert!(node.asset_info().is_ok());
        Ok(())
    })
}

#[test]
fn node_get_message_nodes() -> Result<()> {
    with_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        node.cook_blocking()?;
        let message_nodes = node.get_message_nodes()?;
        let msg_node = message_nodes.first().ok_or_else(|| {
            hapi_rs::HapiError::Internal("hapi_geo has at least one message node".into())
        })?;
        let msg_node = msg_node.to_node(&session)?;
        let msg: String = msg_node.get_cook_result_string(StatusVerbosity::Statusverbosity2)?;
        assert!(msg.contains("Warning generated by Python node"));
        let bits = node.check_for_specific_error(ErrorCode::PythonNodeError as i32)?;
        assert!(bits & (ErrorCode::PythonNodeError as i32) == 0);
        Ok(())
    })
}

#[test]
fn node_output_names() -> Result<()> {
    with_session(|session| {
        session.load_asset_file(HdaFile::Parameters.path())?;
        let node = session.create_node("Object/hapi_parms")?;
        let outputs = node.get_output_names()?;
        assert_eq!(outputs[0], "nothing");
        Ok(())
    })
}

#[test]
fn node_get_parm_with_tag() -> Result<()> {
    with_session(|session| {
        session.load_asset_file(HdaFile::Parameters.path())?;
        let node = session.create_node("Object/hapi_parms")?;
        assert!(node.parameter_with_tag("my_tag")?.is_some());
        assert!(node.parameter_with_tag("no_such_tag")?.is_none());
        Ok(())
    })
}

#[test]
fn node_set_animate_transform() -> Result<()> {
    with_session(|session| {
        let bone = session.create_node("Object/bone")?;
        let ty = [
            KeyFrame {
                time: 0.0,
                value: 0.0,
                in_tangent: 0.0,
                out_tangent: 0.0,
            },
            KeyFrame {
                time: 1.0,
                value: 5.0,
                in_tangent: 0.0,
                out_tangent: 0.0,
            },
        ];
        bone.set_transform_anim_curve(TransformComponent::Ty, &ty)?;
        session.set_time(1.0)?;
        if let Parameter::Float(p) = bone.parameter("ty")? {
            assert_eq!(p.get(1)?, 5.0);
        }
        Ok(())
    })
}

#[test]
fn node_get_set_preset() -> Result<()> {
    with_session(|session| {
        let node = session.create_node("Object/null")?;
        if let Parameter::Float(p) = node.parameter("scale")? {
            assert_eq!(p.get(0)?, 1.0);
            let save = node.get_preset("test", PresetType::Binary)?;
            p.set(0, 2.0)?;
            node.set_preset("test", PresetType::Binary, &save)?;
            assert_eq!(p.get(0)?, 1.0);
        }
        Ok(())
    })
}

#[test]
fn node_wiring_paths_and_cook_metadata() -> Result<()> {
    with_session(|session| {
        let obj = session.create_node("Object/geo")?;
        let sphere = session.node_builder("sphere").with_parent(&obj).create()?;
        let merge = session.node_builder("merge").with_parent(&obj).create()?;
        merge.connect_input(0, &sphere, 0)?;
        assert!(merge.input_node(0)?.is_some());
        merge.disconnect_input(0)?;
        assert!(merge.input_node(0)?.is_none());
        merge.connect_input(0, &sphere, 0)?;
        merge.disconnect_outputs(0)?;
        sphere.rename("sphere_renamed")?;
        assert!(sphere.path()?.contains("sphere_renamed"));
        assert!(!sphere.path()?.is_empty());
        assert!(!obj.get_objects_info()?.is_empty());

        session.load_asset_file(HdaFile::Geometry.path())?;
        let asset = session.create_node("Object/hapi_geo")?;
        asset.cook_blocking()?;
        let _ = asset.get_composed_cook_result_string(StatusVerbosity::Statusverbosity0)?;
        asset.reset_simulation()?;
        Ok(())
    })
}

#[test]
fn node_manager_and_handles() -> Result<()> {
    with_session(|session| {
        assert!(matches!(ManagerType::from_str("Driver")?, ManagerType::Rop));
        assert!(ManagerType::from_str("unknown").is_err());

        let obj_mgr = session.get_manager_node(ManagerType::Obj)?;
        assert!(!obj_mgr.get_children()?.is_empty());
        let _ = obj_mgr.find_network_nodes(NodeType::Obj)?;

        let obj = session.create_node("Object/geo")?;
        let sphere = session.node_builder("sphere").with_parent(&obj).create()?;
        let handle = sphere.handle;
        assert!(handle.path(&session)?.contains("sphere"));
        assert!(!handle.path(&session)?.is_empty());
        assert!(handle.as_geometry_node(&session)?.is_some());
        assert!(obj.handle.as_geometry_node(&session)?.is_none());

        let _: NodeHandle = sphere.clone().into();
        let _: NodeHandle = (&sphere).into();
        let _: Option<NodeHandle> = sphere.clone().into();
        let _: Option<NodeHandle> = (&sphere).into();
        let _: hapi_rs::raw::HAPI_NodeId = handle.into();
        Ok(())
    })
}

#[test]
fn node_top_networks() -> Result<()> {
    with_session_asset(HdaFile::Top, |lib| {
        let asset = lib.try_create_first()?;
        let networks = asset.find_top_networks()?;
        assert!(!networks.is_empty());
        let node = networks[0].clone();
        assert!(node.to_top_node().is_some());
        Ok(())
    })
}

#[test]
fn node_instanced_objects() -> Result<()> {
    with_session_asset(HdaFile::Geometry, |lib| {
        let node = lib.try_create_first()?;
        node.cook_blocking()?;
        let inst = node.get_child_by_path("obj_instance")?.unwrap();
        let ids = inst.get_instanced_object_ids()?;
        assert!(!ids.is_empty());
        Ok(())
    })
}
