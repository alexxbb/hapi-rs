use hapi_rs::{
    Result,
    asset::AssetLibrary,
    attribute::*,
    enums::{AttributeOwner, PartType},
    geometry::{Geometry, PartInfo},
    session::{CookResult, Session, SessionOptions, quick_session},
};
use once_cell::sync::Lazy;

thread_local! {
    static SESSION: Lazy<Session> = Lazy::new(|| {
        let _ = env_logger::try_init();
        let opt = SessionOptions::builder().threaded(true).build();
        quick_session(Some(&opt)).expect("Could not create test session")
    });
}

pub enum Asset {
    Geometry,
    Volume,
    Parameters,
    Spaceship,
}

pub fn hda_file(asset: Asset) -> &'static str {
    match asset {
        Asset::Geometry => "../otls/hapi_geo.hda",
        Asset::Volume => "../otls/hapi_vol.hda",
        Asset::Parameters => "../otls/hapi_parms.hda",
        Asset::Spaceship => "../otls/sesi/SideFX_spaceship.hda",
    }
}

pub fn with_session<F, R>(f: F) -> Result<R>
where
    F: FnOnce(Session) -> Result<R>,
{
    SESSION.with(|session| f((*session).clone()))
}

pub fn with_session_asset<F>(asset: Asset, f: F) -> Result<()>
where
    F: FnOnce(AssetLibrary) -> Result<()>,
{
    let asset_file = hda_file(asset);
    let data = std::fs::read(asset_file)?;
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
    let attr_p = geo
        .add_numeric_attribute::<f32>("P", part.part_id(), info)
        .expect("attr_p");
    attr_p
        .set(
            part.part_id(),
            &[0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
        )
        .expect("set_P");
    geo.set_vertex_list(0, [0, 1, 2]).unwrap();
    geo.set_face_counts(0, [3]).unwrap();
    let info = AttributeInfo::default()
        .with_count(part.point_count())
        .with_tuple_size(1)
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Int);
    let id_attr = geo
        .add_numeric_attribute::<i32>("id", part.part_id(), info)
        .expect("id_attr");
    id_attr.set(0, &[1, 2, 3]).unwrap();

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
        session.load_asset_file(hda_file(Asset::Geometry))?;
        let node = session.create_node("Object/hapi_geo")?;
        let cook_result = node.cook_blocking()?;
        assert_eq!(cook_result, CookResult::Succeeded);
        let geo = node.geometry()?.expect("must have geometry");
        f(geo)
    })
}
