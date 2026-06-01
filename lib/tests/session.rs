use hapi_rs::Result;
use hapi_rs::raw::CacheProperty;
use hapi_rs::server::ServerOptions;
use hapi_rs::session::{
    CookOptions, CookResult, ManagerType, SessionOptions, SessionSyncInfo, TimelineOptions,
    Viewport, new_in_process_session, new_thrift_session,
};
use pretty_assertions::assert_eq;

mod utils;
use utils::with_session;

#[test]
fn session_creation_helpers() -> Result<()> {
    let tmp_file = tempfile::NamedTempFile::new()?;
    let log_file = tmp_file.into_temp_path().keep().expect("keep temp log");
    assert!(
        new_thrift_session(
            SessionOptions::default(),
            ServerOptions::shared_memory_with_defaults().with_log_file(&log_file)
        )?
        .is_valid()
    );
    assert!(
        new_thrift_session(
            SessionOptions::default(),
            ServerOptions::pipe_with_defaults().with_log_file(&log_file),
        )?
        .is_valid()
    );
    assert!(
        new_thrift_session(
            SessionOptions::default(),
            ServerOptions::socket_with_defaults(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::LOCALHOST,
                37777
            ))
            .with_log_file(&log_file),
        )?
        .is_valid()
    );
    assert!(new_in_process_session(None)?.is_valid());
    let _ = std::fs::remove_file(log_file);
    Ok(())
}

#[test]
fn session_get_set_time() -> Result<()> {
    // For some reason, this test randomly fails when using shared session
    let tmp_file = tempfile::NamedTempFile::new()?;
    let log_file = tmp_file.into_temp_path().keep().expect("keep temp log");
    let session = new_thrift_session(
        SessionOptions::default(),
        ServerOptions::shared_memory_with_defaults().with_log_file(&log_file),
    )?;
    let opt = TimelineOptions::default().with_end_time(5.5);
    session.set_timeline_options(&opt)?;
    let opt2 = session.get_timeline_options()?;
    assert!(opt.end_time().eq(&opt2.end_time()));
    session.set_time(4.12)?;
    assert!(matches!(session.cook(), Ok(CookResult::Succeeded)));
    assert_eq!(session.get_time()?, 4.12);
    assert_eq!(CookResult::Succeeded.message(), None);
    assert_eq!(
        CookResult::CookErrors("cook error".into()).message(),
        Some("cook error")
    );
    assert_eq!(
        CookResult::FatalErrors("fatal error".into()).message(),
        Some("fatal error")
    );
    session.cleanup()?;
    let _ = std::fs::remove_file(log_file);
    Ok(())
}

#[test]
fn session_set_viewport() -> Result<()> {
    with_session(|session| {
        let vp = Viewport::default()
            .with_rotation([0.7, 0.7, 0.7, 0.7])
            .with_position([0.0, 1.0, 0.0])
            .with_offset(3.5);
        session.set_viewport(&vp)?;
        let vp2 = session.get_viewport()?;
        assert_eq!(vp.position(), vp2.position());
        assert_eq!(vp.rotation(), vp2.rotation());
        assert_eq!(vp.offset(), vp2.offset());
        Ok(())
    })
}

#[test]
fn session_sync() -> Result<()> {
    with_session(|session| {
        assert!(session.is_valid());
        session.set_sync(true)?;
        let info = SessionSyncInfo::default()
            .with_sync_viewport(true)
            .with_cook_using_houdini_time(true);
        session.set_sync_info(&info)?;
        session.cook()?;
        let info = session.get_sync_info()?;
        assert!(info.sync_viewport());
        assert!(info.cook_using_houdini_time());
        session.set_sync(false)?;
        Ok(())
    })
}

#[test]
fn session_manager_nodes() -> Result<()> {
    with_session(|session| {
        session.get_manager_node(ManagerType::Obj)?;
        session.get_manager_node(ManagerType::Chop)?;
        session.get_manager_node(ManagerType::Cop)?;
        session.get_manager_node(ManagerType::Rop)?;
        session.get_manager_node(ManagerType::Top)?;
        Ok(())
    })
}

#[test]
fn cache_properties() -> Result<()> {
    with_session(|session| {
        let cache_names = session
            .get_active_cache_names()?
            .into_iter()
            .collect::<Vec<_>>();
        assert!(cache_names.contains(&String::from("SOP Cache")));
        assert!(cache_names.contains(&String::from("HDA Contents Cache")));
        session.set_cache_property_value("SOP Cache", CacheProperty::CachepropMax, 2048)?;
        let cache_val =
            session.get_cache_property_value("SOP Cache", CacheProperty::CachepropMax)?;
        assert_eq!(cache_val, 2048);
        Ok(())
    })
}

#[test]
fn test_get_preset_names() -> Result<()> {
    let bytes = std::fs::read("tests/data/bone.idx")?;
    with_session(|session| {
        log::info!("Reading preset file");
        session.get_preset_names(&bytes)?;
        Ok(())
    })
}

#[test]
fn session_hip_timing_and_cook_progress() -> Result<()> {
    with_session(|session| {
        session.create_node("Object/null")?;
        let tmp = tempfile::NamedTempFile::new()?;
        let path = tmp.path();
        session.save_hip(path, false)?;
        session.set_use_houdini_time(true)?;
        assert!(session.get_use_houdini_time()?);
        session.set_use_houdini_time(false)?;
        session.load_hip(path, true)?;
        let hip_id = session.merge_hip(&path.to_string_lossy(), true)?;
        assert!(!session.get_hip_file_nodes(hip_id)?.is_empty());
        session.cook()?;
        let _ = session.get_cook_state_status()?;
        assert!(!session.is_cooking()?);
        let _ = session.get_cook_result_string(hapi_rs::raw::StatusVerbosity::Statusverbosity0)?;
        let _ = session.get_connection_error(false)?;
        let _ = session.cooking_total_count()?;
        let _ = session.cooking_current_count()?;
        session.interrupt()?;
        Ok(())
    })
}

#[test]
fn session_node_builder_and_lookup_helpers() -> Result<()> {
    with_session(|session| {
        let node = session
            .node_builder("Object/null")
            .with_label("labelled_null")
            .cook(true)
            .create()?;
        assert!(node.is_valid()?);

        let path = node.path()?;
        assert!(session.get_node_from_path(&path, None)?.is_some());
        assert!(
            session
                .get_node_from_path("/obj/no_such_node", None)?
                .is_none()
        );

        assert!(
            session
                .find_parameter_from_path("missing_slash", None)?
                .is_none()
        );
        assert!(
            session
                .find_parameter_from_path("/obj/no_such_node/tx", None)?
                .is_none()
        );
        assert!(
            session
                .find_parameter_from_path(format!("{path}/tx"), None)?
                .is_some()
        );

        session.delete_node(node)?;
        Ok(())
    })
}

#[test]
fn session_options_and_compositor_helpers() -> Result<()> {
    let options = SessionOptions::default()
        .houdini_env_files(["/tmp/houdini.env"])
        .otl_search_paths(["/tmp/otls"])
        .dso_search_paths(["/tmp/dso"])
        .image_search_paths(["/tmp/images"])
        .audio_search_paths(["/tmp/audio"])
        .cook_options(CookOptions::default())
        .threaded(false)
        .cleanup(false);
    assert!(!options.threaded);
    assert!(!options.cleanup);

    with_session(|session| {
        session.python_thread_interpreter_lock(true)?;
        session.python_thread_interpreter_lock(false)?;

        let mut options = session.get_compositor_options()?;
        options.set_max_resolution_x(512);
        options.set_max_resolution_y(512);
        session.set_compositor_options(&options)?;
        let options = session.get_compositor_options()?;
        assert_eq!(options.max_resolution_x(), 512);
        assert_eq!(options.max_resolution_y(), 512);

        let profile = session.start_performance_monitor_profile("hapi-rs-test")?;
        let tmp = tempfile::NamedTempFile::new()?;
        session.stop_performance_monitor_profile(profile, tmp.path().to_string_lossy().as_ref())?;
        Ok(())
    })
}
