use once_cell::sync::Lazy;

use hapi_rs::{
    node::{
        HoudiniNode, KeyFrame, NodeFlags, NodeType, PresetType, RSTOrder, TransformComponent,
        TransformEuler,
    },
    parameter::Parameter,
    session::{quick_session, CookResult, Session, SessionOptions},
    Result,
};

static SESSION: Lazy<Session> = Lazy::new(|| {
    env_logger::try_init().ok();
    let opt = SessionOptions::builder().threaded(true).build();
    let session = quick_session(Some(&opt)).expect("Could not create test session");
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
    session
});

#[test]
fn node_create() {
    let node = SESSION.create_node("Object/spaceship").unwrap();
    assert_eq!(
        node.cook_count(NodeType::None, NodeFlags::None, true)
            .unwrap(),
        0
    );
    node.cook().unwrap(); // in threaded mode always successful
    assert_eq!(
        node.cook_count(NodeType::None, NodeFlags::None, true)
            .unwrap(),
        1
    );
    assert!(matches!(SESSION.cook().unwrap(), CookResult::Succeeded));
}

#[test]
fn node_set_node_display_flag() -> Result<()> {
    let sop = SESSION.create_node("Object/geo").unwrap();
    let sphere = SESSION.node_builder("sphere").with_parent(&sop).create()?;
    let cube = SESSION.node_builder("box").with_parent(&sop).create()?;
    sphere.cook().unwrap();
    cube.cook().unwrap();
    cube.set_display_flag(true)?;
    SESSION.cook().expect("session cook");
    assert!(!sphere.geometry()?.unwrap().geo_info()?.is_display_geo());
    sphere.set_display_flag(true)?;
    assert!(!cube.geometry()?.unwrap().geo_info()?.is_display_geo());
    Ok(())
}

#[test]
fn node_inputs_and_outputs() {
    let node = SESSION.create_node("Object/hapi_geo").unwrap();
    let geo = node.geometry().unwrap().unwrap();
    let mut input = geo.node.input_node(0).unwrap();
    while let Some(ref n) = input {
        assert!(n.is_valid().unwrap());
        input = n.input_node(0).unwrap();
    }
    let outputs = geo.node.output_connected_nodes(0, false).unwrap();
    assert!(outputs.is_empty());
    let n = node
        .get_child_by_path("geo/point_attr")
        .unwrap()
        .expect("child node");
    assert_eq!(
        n.get_input_name(0).unwrap(),
        "Geometry to Process with Wrangle"
    );
}

#[test]
fn node_search() {
    let asset = SESSION.create_node("Object/hapi_geo").unwrap();
    asset.cook_blocking().unwrap();
    let nope = asset.get_child_by_path("bla").unwrap();
    assert!(nope.is_none());
    asset
        .get_child_by_path("geo/add_color")
        .unwrap()
        .expect("color node");
    asset
        .find_child_node("add_color", true)
        .unwrap()
        .expect("color node");
    let children = asset
        .find_children_by_type(NodeType::Sop, NodeFlags::Display, true)
        .unwrap();
    assert_ne!(children.len(), 0)
}

#[test]
fn node_transform() {
    let obj = SESSION.create_node("Object/null").unwrap();
    let t = obj.get_transform(None, None).unwrap();
    assert_eq!(t.position(), [0.0, 0.0, 0.0]);
    assert_eq!(t.scale(), [1.0, 1.0, 1.0]);
    assert_eq!(t.rst_order(), RSTOrder::Default);
    obj.set_transform(
        &TransformEuler::default()
            .with_position([0.0, 1.0, 0.0])
            .with_rotation([45.0, 0.0, 0.0]),
    )
    .unwrap();
    obj.cook().unwrap();
    assert!(obj.get_object_info().unwrap().has_transform_changed());
    let t = obj.get_transform(None, None).unwrap();
    assert_eq!(t.position(), [0.0, 1.0, 0.0]);
    SESSION
        .get_composed_object_transform(obj.parent_node().unwrap(), RSTOrder::Default)
        .unwrap();
}

#[test]
fn node_save_and_load() {
    let cam = SESSION.create_node("Object/cam").unwrap();
    let tmp = std::env::temp_dir().join("node");
    cam.save_to_file(&tmp).expect("save_to_file");
    let new = HoudiniNode::load_from_file(&SESSION, None, "loaded_cam", true, &tmp)
        .expect("load_from_file");
    std::fs::remove_file(&tmp).unwrap();
    cam.delete().unwrap();
    new.delete().unwrap();
}

#[test]
fn node_number_of_geo_outputs() {
    let node = SESSION.create_node("Object/hapi_geo").unwrap();
    assert_eq!(node.number_of_geo_outputs().unwrap(), 2);
    let infos = node.geometry_output_nodes().unwrap();
    assert_eq!(infos.len(), 2);
}

#[test]
fn node_output_names() {
    let node = SESSION.create_node("Object/hapi_parms").unwrap();
    let outputs = node.get_output_names().unwrap();
    assert_eq!(outputs[0], "nothing");
}

#[test]
fn node_get_parm_with_tag() {
    let node = SESSION.create_node("Object/hapi_parms").unwrap();
    assert!(node.parameter_with_tag("my_tag").unwrap().is_some());
}

#[test]
fn node_set_animate_transform() {
    let bone = SESSION.create_node("Object/bone").unwrap();
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
    bone.set_transform_anim_curve(TransformComponent::Ty, &ty)
        .unwrap();
    SESSION.set_time(1.0).unwrap();
    if let Parameter::Float(p) = bone.parameter("ty").unwrap() {
        assert_eq!(p.get(1).unwrap(), 5.0);
    }
}

#[test]
fn node_get_set_preset() {
    let node = SESSION.create_node("Object/null").unwrap();
    if let Parameter::Float(p) = node.parameter("scale").unwrap() {
        assert_eq!(p.get(0).unwrap(), 1.0);
        let save = node.get_preset("test", PresetType::Binary).unwrap();
        p.set(0, 2.0).unwrap();
        node.set_preset("test", PresetType::Binary, &save).unwrap();
        assert_eq!(p.get(0).unwrap(), 1.0);
    }
}
