use once_cell::sync::Lazy;

use hapi_rs::{
    geometry::Materials,
    session::{quick_session, Session, SessionOptions},
};

static SESSION: Lazy<Session> = Lazy::new(|| {
    env_logger::init();
    let opt = SessionOptions::builder().threaded(true).build();
    let session = quick_session(Some(&opt)).expect("Could not create test session");
    session
        .load_asset_file("otls/sesi/SideFX_spaceship.hda")
        .expect("load asset");
    session
});

#[test]
fn image_file_formats() {
    let formats = SESSION.get_supported_image_formats().unwrap();
    assert!(formats.iter().any(|f| f.name().unwrap() == "JPEG"));
    assert!(formats.iter().any(|f| f.extension().unwrap() == "jpg"));
}

#[test]
fn image_extract_api() {
    let node = SESSION.create_node("Object/spaceship").unwrap();
    node.cook_blocking().unwrap();
    let geo = node.geometry().expect("geometry").unwrap();
    let part = geo.part_info(0).unwrap();
    let mats = geo.get_materials(&part).unwrap().expect("materials");
    if let Materials::Single(mat) = mats {
        let mut info = mat.get_image_info().unwrap();
        info.set_x_res(512);
        info.set_y_res(512);
        mat.render_texture("baseColorMap").unwrap();
        mat.set_image_info(&info).unwrap();
        let ip = mat.get_image_planes().unwrap();
        assert!(ip.iter().any(|ip| *ip == "C"));
        let file = std::env::temp_dir().join("hapi.jpeg");
        mat.extract_image_to_file("C", file).expect("extract_image");
        mat.render_texture("baseColorMap").unwrap();
        let mut bytes = vec![];
        mat.extract_image_to_memory(&mut bytes, "C", "JPEG")
            .expect("extract_image");
        assert!(!bytes.is_empty());
    } else {
        panic!("Failed to extract material data")
    }
}
