#![allow(dead_code)]
use hapi_rs::server::{LicensePreference, ServerOptions};
use hapi_rs::{
    HapiError, Result,
    asset::AssetLibrary,
    attribute::*,
    enums::{AttributeOwner, PartType},
    geometry::{Geometry, PartInfo},
    session::{CookResult, Session, SessionOptions, new_thrift_session},
};

use std::path::PathBuf;

struct TempLogFiles(pub(crate) Vec<PathBuf>);

impl Drop for TempLogFiles {
    fn drop(&mut self) {
        for path in self.0.iter() {
            let _ = std::fs::remove_file(path);
        }
    }
}

thread_local! {
    // Keep track of temp log files so they can be removed when the thread local is dropped.
    // We redirect HARS output to a log file because it can be chatty in stdout.
    static TEMP_LOG_FILES: std::sync::LazyLock<std::sync::Mutex<TempLogFiles>> = std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(TempLogFiles(vec![]))
    });
    static SESSION: std::sync::LazyLock<Session> = std::sync::LazyLock::new(|| {
        let _ = env_logger::try_init();
        let temp_log_file = tempfile::NamedTempFile::new().unwrap();
        let temp_log_file_path = temp_log_file.into_temp_path().keep().unwrap();
        TEMP_LOG_FILES.with(|temp_log_files| {
            temp_log_files.lock().unwrap().0.push(temp_log_file_path.clone());
        });
        let server_options = ServerOptions::shared_memory_with_defaults().with_log_file(temp_log_file_path)
        .with_license_preference(LicensePreference::HoudiniEngineAndCore);
        new_thrift_session(SessionOptions::default(), server_options)
            .expect("Could not create test session")
    });

    #[cfg(feature = "async-cooking")]
    static ASYNC_SESSION: std::sync::LazyLock<Session> = std::sync::LazyLock::new(|| {
        let _ = env_logger::try_init();
        let mut session_info = SessionInfo::default();
        // For async attribute access connection_count must be > 0 according to SESI support, otherwise HARS crashes.
        println!("FIXME: H21.0 has a bug around connection count. Async tests are disabled for now.");
        session_info.set_connection_count(2);
        let opt = SessionOptions {
            threaded: true,
            ..Default::default()
        };
        let server_options = ServerOptions::shared_memory_with_defaults().with_license_preference(LicensePreference::HoudiniEngineAndCore);
        new_thrift_session(opt, server_options).expect("Could not create async test session")
    });
}

pub enum HdaFile {
    Geometry,
    Volume,
    Parameters,
    Spaceship,
}

impl HdaFile {
    pub fn path(&self) -> &'static str {
        match self {
            HdaFile::Geometry => "../otls/hapi_geo.hda",
            HdaFile::Volume => "../otls/hapi_vol.hda",
            HdaFile::Parameters => "../otls/hapi_parms.hda",
            HdaFile::Spaceship => "../otls/sesi/SideFX_spaceship.hda",
        }
    }
}

pub fn with_session<F, R>(f: F) -> Result<R>
where
    F: FnOnce(Session) -> Result<R>,
{
    SESSION.with(|session| f((*session).clone()))
}

#[cfg(feature = "async-cooking")]
pub fn with_async_session<F, R>(f: F) -> Result<R>
where
    F: FnOnce(Session) -> Result<R>,
{
    ASYNC_SESSION.with(|session| f((*session).clone()))
}

pub fn with_session_asset<F>(hda_file: HdaFile, f: F) -> Result<()>
where
    F: FnOnce(AssetLibrary) -> Result<()>,
{
    let data = std::fs::read(hda_file.path())?;
    with_session(|session| f(AssetLibrary::from_memory(session, &data)?))
}

pub fn create_triangle(session: &Session) -> Result<Geometry> {
    let geo = session.create_input_node("triangle", None)?;
    let part = PartInfo::default()
        .with_part_type(PartType::Mesh)
        .with_face_count(1)
        .with_point_count(3)
        .with_vertex_count(3);
    geo.set_part_info(&part)?;
    let info = AttributeInfo::default()
        .with_count(part.point_count())
        .with_tuple_size(3)
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Float);
    let attr_p = geo.add_numeric_attribute::<f32>("P", part.part_id(), info)?;
    attr_p.set(
        part.part_id(),
        &[0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
    )?;
    geo.set_vertex_list(0, [0, 1, 2])?;
    geo.set_face_counts(0, [3])?;
    let info = AttributeInfo::default()
        .with_count(part.point_count())
        .with_tuple_size(1)
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Int);
    let id_attr = geo.add_numeric_attribute::<i32>("id", part.part_id(), info)?;
    id_attr.set(0, &[1, 2, 3])?;

    geo.commit()?;
    geo.node.cook_blocking()?;
    Ok(geo)
}

pub fn create_single_point_geo(session: &Session) -> Result<Geometry> {
    let geo = session.create_input_node("dummy", None)?;
    let part = PartInfo::default()
        .with_part_type(PartType::Mesh)
        .with_point_count(1);
    geo.set_part_info(&part)?;
    let p_info = AttributeInfo::default()
        .with_count(part.point_count())
        .with_tuple_size(3)
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Float);
    let id_attr = geo.add_numeric_attribute::<f32>("P", part.part_id(), p_info)?;
    id_attr.set(part.part_id(), &[0.0, 0.0, 0.0])?;
    geo.commit()?;
    geo.node.cook_blocking()?;
    Ok(geo)
}

pub fn with_test_geometry<F>(f: F) -> Result<()>
where
    F: FnOnce(Geometry) -> Result<()>,
{
    SESSION.with(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        let cook_result = node.cook_blocking()?;
        if cook_result != CookResult::Succeeded {
            return Err(HapiError::Internal(format!(
                "expected cook to succeed, got {cook_result:?}"
            )));
        }
        let geo = node
            .geometry()?
            .ok_or_else(|| HapiError::Internal("must have geometry".into()))?;
        f(geo)
    })
}
