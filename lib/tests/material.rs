use hapi_rs::Result;
use hapi_rs::geometry::Materials;

mod utils;

use utils::{HdaFile, with_session};

#[test]
fn image_file_formats() -> Result<()> {
    with_session(|session| {
        let formats = session.get_supported_image_formats()?;
        assert!(formats.iter().any(|f| f.name().is_ok_and(|n| n == "JPEG")));
        assert!(
            formats
                .iter()
                .any(|f| f.extension().is_ok_and(|e| e == "jpg"))
        );
        Ok(())
    })
}

#[test]
fn image_extract_api() -> Result<()> {
    with_session(|session| {
        session.load_asset_file(HdaFile::Spaceship.path())?;
        let node = session.create_node("Object/spaceship")?;
        node.cook_blocking()?;
        let geo = node
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("geometry".into()))?;
        let part = geo.part_info(0)?;
        let mats = geo
            .get_materials(&part)?
            .ok_or_else(|| hapi_rs::HapiError::Internal("materials".into()))?;
        if let Materials::Single(mat) = mats {
            let mut info = mat.get_image_info()?;
            info.set_x_res(512);
            info.set_y_res(512);
            mat.render_texture("baseColorMap")?;
            mat.set_image_info(&info)?;
            let ip = mat.get_image_planes()?;
            assert!(ip.iter().any(|ip| *ip == "C"));
            let file = std::env::temp_dir().join("hapi.jpeg");
            mat.extract_image_to_file("C", file)?;
            mat.render_texture("baseColorMap")?;
            let mut bytes = vec![];
            mat.extract_image_to_memory(&mut bytes, "C", "JPEG")?;
            assert!(!bytes.is_empty());
        } else {
            return Err(hapi_rs::HapiError::Internal(
                "Failed to extract material data".into(),
            ));
        }
        Ok(())
    })
}
