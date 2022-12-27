use super::*;
use crate::session::tests::with_session;

#[test]
fn node_parameters() {
    with_session(|session| {
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
        if let Parameter::Float(p) = node.parameter("single_float").unwrap() {
            p.set_expression("$T", 0).unwrap();
            assert_eq!("$T", p.expression(0).unwrap().unwrap());
        }

        if let Parameter::String(p) = node.parameter("multi_string").unwrap() {
            let value = p.get_array().unwrap();
            assert_eq!(vec!["foo 1", "bar 2", "baz 3"], value);
            p.set(1, "cheese").unwrap();
            assert_eq!("cheese", p.get(1).unwrap());
        }

        if let Parameter::Int(p) = node.parameter("ord_menu").unwrap() {
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
    });
}

#[test]
fn set_anim_curve() {
    use crate::ffi::KeyFrame;

    with_session(|session| {
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
                    value: 1.0,
                    in_tangent: 0.0,
                    out_tangent: 0.0,
                },
            ];

            p.set_anim_curve(0, &keys).expect("set_anim_curve")
        }
    });
}
