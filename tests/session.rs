use once_cell::sync::Lazy;

use hapi_rs::session::{
    quick_session, ConnectionType, CookResult, ManagerType, Session, SessionOptions,
    SessionSyncInfo, TimelineOptions, Viewport,
};

static SESSION: Lazy<Session> = Lazy::new(|| {
    env_logger::init();
    let session = quick_session(None).expect("Could not create test session");
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
fn session_init_and_teardown() {
    let opt = SessionOptions::builder()
        .dso_search_paths(["/path/one", "/path/two"])
        .otl_search_paths(["/path/thee", "/path/four"])
        .build();
    let ses = quick_session(Some(&opt)).unwrap();
    assert!(matches!(
        ses.connection_type(),
        ConnectionType::ThriftPipe(_)
    ));
    assert!(ses.is_initialized());
    assert!(ses.is_valid());
    assert!(ses.cleanup().is_ok());
    assert!(!ses.is_initialized());
}

#[test]
fn session_get_set_time() {
    // For some reason, this test randomly fails when using shared session
    let session = quick_session(None).expect("Could not start session");
    let _lock = session.lock();
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
    // Starting a new separate session because getting/setting env variables from multiple
    // clients ( threads ) breaks the server
    let session = quick_session(None).expect("Could not start session");
    session.set_server_var::<str>("FOO", "foo_string").unwrap();
    assert_eq!(session.get_server_var::<str>("FOO").unwrap(), "foo_string");
    session.set_server_var::<i32>("BAR", &123).unwrap();
    assert_eq!(session.get_server_var::<i32>("BAR").unwrap(), 123);
    assert!(!session.get_server_variables().unwrap().is_empty());
}

#[test]
fn session_set_viewport() {
    let vp = Viewport::default()
        .with_rotation([0.7, 0.7, 0.7, 0.7])
        .with_position([0.0, 1.0, 0.0])
        .with_offset(3.5);
    SESSION.set_viewport(&vp).expect("set_viewport");
    let vp2 = SESSION.get_viewport().expect("get_viewport");
    assert_eq!(vp.position(), vp2.position());
    assert_eq!(vp.rotation(), vp2.rotation());
    assert_eq!(vp.offset(), vp2.offset());
}

#[test]
fn session_sync() {
    let info = SessionSyncInfo::default()
        .with_sync_viewport(true)
        .with_cook_using_houdini_time(true);
    SESSION.set_sync_info(&info).unwrap();
    SESSION.cook().unwrap();
    let info = SESSION.get_sync_info().unwrap();
    assert!(info.sync_viewport());
    assert!(info.cook_using_houdini_time());
}

#[test]
fn session_manager_nodes() {
    SESSION.get_manager_node(ManagerType::Obj).unwrap();
    SESSION.get_manager_node(ManagerType::Chop).unwrap();
    SESSION.get_manager_node(ManagerType::Cop).unwrap();
    SESSION.get_manager_node(ManagerType::Rop).unwrap();
    SESSION.get_manager_node(ManagerType::Top).unwrap();
}
