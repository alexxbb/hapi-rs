use once_cell::sync::Lazy;

use hapi_rs::{
    parameter::{KeyFrame, Parameter, ParmBaseTrait, ParmType},
    session::{quick_session, Session},
    Result,
};

thread_local! {
    static SESSION: Lazy<Session> = Lazy::new(|| {
        env_logger::try_init().ok();
        let session = quick_session(None).expect("Could not create test session");
        session
            .load_asset_file("otls/hapi_parms.hda")
            .expect("load asset");
        session
    });
}

#[test]
fn parameters_get_set() {
    SESSION.with(|session| {
        let node = session
            .create_node("Object/hapi_parms")
            .expect("create_node");
        for p in node.parameters().unwrap() {
            assert!(p.name().is_ok());
        }
        if let Parameter::Float(p) = node.parameter("color").unwrap() {
            let val = p.get_array().unwrap();
            assert_eq!(&val, &[0.55f32, 0.75, 0.95]);
            p.set_array([0.7, 0.5, 0.3]).unwrap();
            let val = p.get_array().unwrap();
            assert_eq!(&val, &[0.7f32, 0.5, 0.3]);
        }

        if let Parameter::String(p) = node.parameter("multi_string").unwrap() {
            let value = p.get_array().unwrap();
            assert_eq!(vec!["foo 1", "bar 2", "baz 3"], value);
            p.set(1, "cheese").unwrap();
            assert_eq!("cheese", p.get(1).unwrap());
        }

        let menu_folder = node.parameter("folder1_0").unwrap();
        assert_eq!(menu_folder.info().parm_type(), ParmType::Folder);
        if let ref parm @ Parameter::Int(ref p) = node.parameter("ord_menu").unwrap() {
            assert_eq!(
                parm.parent().unwrap().unwrap().id(),
                menu_folder.info().id()
            );
            assert!(p.is_menu());
            assert_eq!(p.get(0).unwrap(), 0);
            if let Some(items) = p.menu_items().unwrap() {
                assert_eq!(items[0].value().unwrap(), "foo");
                assert_eq!(items[0].label().unwrap(), "Foo");
            }
        }

        if let Parameter::String(p) = node.parameter("script_menu").unwrap() {
            assert!(p.is_menu());
            assert_eq!(p.get(0).unwrap(), "rs");
            if let Some(items) = p.menu_items().unwrap() {
                assert_eq!(items[0].value().unwrap(), "rs");
                assert_eq!(items[0].label().unwrap(), "Rust");
            }
        }

        if let Parameter::Int(p) = node.parameter("toggle").unwrap() {
            assert_eq!(p.get(0).unwrap(), 0);
            p.set(0, 1).unwrap();
            assert_eq!(p.get(0).unwrap(), 1);
        }

        // test button callback
        if let Parameter::Button(ip) = node.parameter("button").unwrap() {
            if let Parameter::String(sp) = node.parameter("single_string").unwrap() {
                assert_eq!(sp.get(0).unwrap(), "hello");
                ip.press_button().unwrap();
                assert_eq!(sp.get(0).unwrap(), "set from callback");
            }
        }
    })
}

#[test]
fn parameters_set_anim_expression() {
    SESSION.with(|session| {
        let node = session.create_node("Object/null").unwrap();

        if let Ok(Parameter::Float(p)) = node.parameter("scale") {
            let keys = vec![
                KeyFrame {
                    time: 0.0,
                    value: 0.0,
                    in_tangent: 0.0,
                    out_tangent: 0.0,
                },
                KeyFrame {
                    time: 1.0,
                    value: 3.0,
                    in_tangent: 0.0,
                    out_tangent: 0.0,
                },
            ];

            p.set_anim_curve(0, &keys).expect("set_anim_curve");
            assert!(p.has_expression(0).unwrap());
            session.set_time(1.0).unwrap();
            assert_eq!(p.get(0).unwrap(), 3.0);
            p.remove_expression(0).unwrap();
            assert!(!p.has_expression(0).unwrap());
            assert_eq!(p.expression(0).unwrap(), None);
            p.set_expression("$T", 0).unwrap();
            assert_eq!(p.expression(0).unwrap().as_deref(), Some("$T"));
            session.set_time(10.0).unwrap();
            assert_eq!(p.get(0).unwrap(), 10.0);
            p.remove_expression(0).unwrap();
            assert_eq!(p.expression(0).unwrap(), None);
        }
    })
}

#[test]
fn parameters_reset_to_default() {
    SESSION.with(|session| {
        let node = session
            .create_node("Object/hapi_parms")
            .expect("create_node");
        let parm = node.parameter("single_float").unwrap();
        if let Parameter::Float(p) = parm {
            let default = p.get(0).unwrap();
            p.set(0, 0.01).unwrap();
            p.revert_to_default(Some(0)).unwrap();
            assert_eq!(p.get(0).unwrap(), default);
        }
    })
}

#[test]
fn parameter_tags() {
    SESSION.with(|session| {
        let node = session
            .create_node("Object/hapi_parms")
            .expect("create_node");
        if let Ok(Parameter::Button(parm)) = node.parameter("button") {
            assert!(parm.has_tag("script_callback").unwrap());
            let tag_name = parm.get_tag_name(0).unwrap();
            assert_eq!(tag_name, "script_callback_language");
            let tag_value = parm.get_tag_value("script_callback_language").unwrap();
            assert_eq!(tag_value, "python");
        }
    })
}

#[test]
fn parameters_save_parm_file() {
    SESSION.with(|session| {
        let node = session
            .create_node("Object/hapi_parms")
            .expect("create_node");

        if let Parameter::String(geo_parm) = node.parameter("geo_file").unwrap() {
            let dir = tempfile::TempDir::new().unwrap();
            let dir = dir.into_path();
            let filename = "geo.bgeo.sc";
            geo_parm.save_parm_file(&dir, filename).unwrap();
            let filesize = std::fs::metadata(dir.join(filename)).unwrap().len();
            assert!(filesize > 1024);
            std::fs::remove_dir_all(dir).unwrap();
        }
    })
}

#[test]
fn get_set_value_as_node() {
    SESSION.with(|session| {
        let node = session
            .create_node("Object/hapi_parms")
            .expect("create_node");
        let Ok(Parameter::String(parm)) = node.parameter("op_path") else {
            panic!("op_node string parameter not found");
        };
        assert_eq!(parm.info().parm_type(), ParmType::Node);
        let null_node = session.create_node("Object/null").unwrap();
        parm.set_value_as_node(&null_node)
            .expect("op_path parameter set");
        let value = parm
            .get_value_as_node()
            .unwrap()
            .expect("op_path node not found");
        assert_eq!(null_node.handle, value);
    })
}

#[test]
fn parameters_concurrent_access() -> Result<()> {
    // This is a dumb test of accessing parameters randomly from multiple threads
    // HAPI claims each session is protected with a lock....
    fn set_parm_value(parm: &Parameter) -> Result<()> {
        match parm {
            Parameter::Float(parm) => {
                let val: [f32; 3] = std::array::from_fn(|_| fastrand::f32());
                parm.set_array(val)?
            }
            Parameter::Int(parm) => {
                let values: Vec<_> = std::iter::repeat_with(|| fastrand::i32(0..10))
                    .take(parm.size() as usize)
                    .collect();
                parm.set_array(values)?
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
                parm.set_array(values)?
            }
            Parameter::Button(parm) => parm.press_button()?,
            Parameter::Other(_) => {}
        };
        Ok(())
    }

    fn get_parm_value(parm: &Parameter) -> Result<()> {
        match parm {
            Parameter::Float(parm) => {
                parm.get(0)?;
            }
            Parameter::Int(parm) => {
                parm.get(0)?;
            }
            Parameter::String(parm) => {
                parm.get(0)?;
            }
            Parameter::Button(_) => {}
            Parameter::Other(_) => {}
        };
        Ok(())
    }

    SESSION.with(|session| {
        // let session = quick_session(Some(
        //     &SessionOptionsBuilder::default().threaded(true).build(),
        // ))?;
        let node = session
            .load_asset_file("otls/hapi_parms.hda")?
            .try_create_first()?;
        node.cook_blocking()?;
        let parameters = node.parameters()?;
        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for _ in 0..3 {
                let handle = scope.spawn(|| -> Result<()> {
                    for _ in 0..parameters.len() {
                        let i = fastrand::usize(..parameters.len());
                        let parm = &parameters[i];
                        if fastrand::bool() {
                            set_parm_value(parm)?;
                            node.cook()?;
                        } else {
                            get_parm_value(parm)?;
                            node.cook()?;
                        }
                    }
                    Ok(())
                });
                handles.push(handle);
            }
            for h in handles {
                h.join().unwrap().unwrap();
            }
        });

        Ok(())
    })
}
