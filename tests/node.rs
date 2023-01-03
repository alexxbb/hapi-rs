use once_cell::sync::Lazy;

use hapi_rs::{
    node::{
        HoudiniNode, KeyFrame, NodeFlags, NodeType, PresetType, RSTOrder, TransformComponent,
        TransformEuler,
    },
    parameter::{Parameter, ParmBaseTrait},
    session::{quick_session, CookResult, Session, SessionOptions, SessionOptionsBuilder},
    Result,
};

static SESSION: Lazy<Session> = Lazy::new(|| {
    env_logger::init();
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
    let infos = node.geometry_outputs().unwrap();
    assert_eq!(infos.len(), 2);
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

#[test]
fn node_parm_concurrent_access() {
    fn set_parm_value(parm: &Parameter) {
        match parm {
            Parameter::Float(parm) => {
                let val: [f32; 3] = std::array::from_fn(|_| fastrand::f32());
                parm.set_array(val).unwrap()
            }
            Parameter::Int(parm) => {
                let values: Vec<_> = std::iter::repeat_with(|| fastrand::i32(0..10))
                    .take(parm.size() as usize)
                    .collect();
                parm.set_array(values).unwrap()
            }
            Parameter::String(parm) => {
                let values: Vec<String> = (0..parm.size())
                    .into_iter()
                    .map(|_| {
                        std::iter::repeat_with(fastrand::alphanumeric)
                            .take(10)
                            .collect()
                    })
                    .collect();
                parm.set_array(values).unwrap()
            }
            Parameter::Button(parm) => parm.press_button().unwrap(),
            Parameter::Other(_) => {}
        };
    }

    fn get_parm_value(parm: &Parameter) {
        match parm {
            Parameter::Float(parm) => {
                parm.get(0).unwrap();
            }
            Parameter::Int(parm) => {
                parm.get(0).unwrap();
            }
            Parameter::String(parm) => {
                parm.get(0).unwrap();
            }
            Parameter::Button(_) => {}
            Parameter::Other(_) => {}
        };
    }

    let session = quick_session(Some(
        &SessionOptionsBuilder::default().threaded(true).build(),
    ))
    .unwrap();
    let lib = session
        .load_asset_file("otls/hapi_parms.hda")
        .expect("loaded asset");
    let node = lib.try_create_first().expect("hapi_parm node");
    node.cook_blocking().unwrap();
    let parameters = node.parameters().expect("parameters");
    std::thread::scope(|scope| {
        for _ in 0..3 {
            scope.spawn(|| {
                for _ in 0..parameters.len() {
                    let i = fastrand::usize(..parameters.len());
                    let parm = &parameters[i];
                    if fastrand::bool() {
                        set_parm_value(parm);
                        node.cook().unwrap();
                    } else {
                        get_parm_value(parm);
                        node.cook().unwrap();
                    }
                }
            });
        }
    });
}
