use hapi_rs::raw::CacheProperty;
use hapi_rs::server::ServerOptions;
use hapi_rs::session::{
    CookResult, License, ManagerType, SessionOptions, SessionSyncInfo, TimelineOptions, Viewport,
    new_thrift_session,
};

mod utils;
use utils::with_session;

#[test]
fn session_get_set_time() {
    // For some reason, this test randomly fails when using shared session
    let session = new_thrift_session(
        SessionOptions::default(),
        ServerOptions::shared_memory_with_defaults(),
    )
    .expect("Could not start session");
    // let _lock = session.lock();
    let opt = TimelineOptions::default().with_end_time(5.5);
    assert!(session.set_timeline_options(opt.clone()).is_ok());
    let opt2 = session.get_timeline_options().expect("timeline_options");
    assert!(opt.end_time().eq(&opt2.end_time()));
    session.set_time(4.12).expect("set_time");
    assert!(matches!(session.cook(), Ok(CookResult::Succeeded)));
    assert_eq!(session.get_time().expect("get_time"), 4.12);
}

#[test]
fn session_server_variables() {
    let session = new_thrift_session(
        SessionOptions::default(),
        ServerOptions::shared_memory_with_defaults()
            .with_env_variables([("HAPI_RS_TEST", "hapi_rs_is_awesome")].iter()),
    )
    .expect("Could not start session");
    session.set_server_var::<str>("FOO", "foo_string").unwrap();
    assert_eq!(session.get_server_var::<str>("FOO").unwrap(), "foo_string");
    session.set_server_var::<i32>("BAR", &123).unwrap();
    assert_eq!(session.get_server_var::<i32>("BAR").unwrap(), 123);
    assert!(!session.get_server_variables().unwrap().is_empty());
    assert_eq!(
        session.get_server_var::<str>("HAPI_RS_TEST").unwrap(),
        "hapi_rs_is_awesome"
    );
    // Make sure the implementation doesn't leak the environment variables to the main process
    assert!(!std::env::vars().any(|(k, _)| k == "HAPI_RS_TEST"));
}

#[test]
fn session_set_viewport() {
    with_session(|session| {
        let vp = Viewport::default()
            .with_rotation([0.7, 0.7, 0.7, 0.7])
            .with_position([0.0, 1.0, 0.0])
            .with_offset(3.5);
        session.set_viewport(&vp).expect("set_viewport");
        let vp2 = session.get_viewport().expect("get_viewport");
        assert_eq!(vp.position(), vp2.position());
        assert_eq!(vp.rotation(), vp2.rotation());
        assert_eq!(vp.offset(), vp2.offset());
        Ok(())
    })
    .unwrap()
}

#[test]
fn session_sync() {
    with_session(|session| {
        assert!(session.is_valid());
        let info = SessionSyncInfo::default()
            .with_sync_viewport(true)
            .with_cook_using_houdini_time(true);
        session.set_sync_info(&info).unwrap();
        session.cook().unwrap();
        let info = session.get_sync_info().unwrap();
        assert!(info.sync_viewport());
        assert!(info.cook_using_houdini_time());
        Ok(())
    })
    .unwrap()
}

#[test]
fn session_manager_nodes() {
    with_session(|session| {
        session.get_manager_node(ManagerType::Obj).unwrap();
        session.get_manager_node(ManagerType::Chop).unwrap();
        session.get_manager_node(ManagerType::Cop).unwrap();
        session.get_manager_node(ManagerType::Rop).unwrap();
        session.get_manager_node(ManagerType::Top).unwrap();
        Ok(())
    })
    .unwrap()
}

#[test]
fn cache_properties() {
    with_session(|session| {
        let cache_names = session
            .get_active_cache_names()
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        assert!(cache_names.contains(&String::from("SOP Cache")));
        assert!(cache_names.contains(&String::from("HDA Contents Cache")));
        session
            .set_cache_property_value("SOP Cache", CacheProperty::CachepropMax, 2048)
            .unwrap();
        let cache_val = session
            .get_cache_property_value("SOP Cache", CacheProperty::CachepropMax)
            .unwrap();
        assert_eq!(cache_val, 2048);
        Ok(())
    })
    .unwrap()
}

#[test]
fn test_license_set_via_environment() {
    let env = [(
        "HOUDINI_PLUGIN_LIC_OPT",
        "--check-licenses=Houdini-Escape --skip-licenses=Houdini-Engine",
    )];

    let server_options =
        ServerOptions::shared_memory_with_defaults().with_env_variables(env.iter());
    let session = new_thrift_session(SessionOptions::default(), server_options)
        .expect("Could not start session");
    let plugin_lic_opt = session.get_server_var::<str>(env[0].0).unwrap();
    session.create_node("Object/null").unwrap();
    let license_type = session.get_license_type().unwrap();
    assert_eq!(plugin_lic_opt, env[0].1.to_string());
    assert_eq!(license_type, License::LicenseHoudini);
}

#[test]
fn test_get_preset_names() {
    let bytes = std::fs::read("tests/data/bone.idx").expect("read file");
    with_session(|session| {
        log::info!("Reading preset file");
        session.get_preset_names(&bytes).expect("2 presets");
        Ok(())
    })
    .unwrap()
}
