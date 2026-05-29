use hapi_rs::Result;
use hapi_rs::raw::CacheProperty;
use hapi_rs::server::ServerOptions;
use hapi_rs::session::{
    CookResult, ManagerType, SessionOptions, SessionSyncInfo, TimelineOptions, Viewport,
    new_thrift_session,
};
use pretty_assertions::assert_eq;

mod utils;
use utils::with_session;

#[test]
fn session_get_set_time() -> Result<()> {
    // For some reason, this test randomly fails when using shared session
    let session = new_thrift_session(
        SessionOptions::default(),
        ServerOptions::shared_memory_with_defaults(),
    )?;
    let opt = TimelineOptions::default().with_end_time(5.5);
    session.set_timeline_options(&opt)?;
    let opt2 = session.get_timeline_options()?;
    assert!(opt.end_time().eq(&opt2.end_time()));
    session.set_time(4.12)?;
    assert!(matches!(session.cook(), Ok(CookResult::Succeeded)));
    assert_eq!(session.get_time()?, 4.12);
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
        let info = SessionSyncInfo::default()
            .with_sync_viewport(true)
            .with_cook_using_houdini_time(true);
        session.set_sync_info(&info)?;
        session.cook()?;
        let info = session.get_sync_info()?;
        assert!(info.sync_viewport());
        assert!(info.cook_using_houdini_time());
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
        let _ = session.cooking_total_count()?;
        let _ = session.cooking_current_count()?;
        Ok(())
    })
}
