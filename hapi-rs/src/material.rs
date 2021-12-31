use crate::ffi::ImageFileFormat;
use crate::ffi::{raw::HAPI_MaterialInfo, ImageInfo};
use crate::node::NodeHandle;
use crate::parameter::ParmHandle;
use crate::session::Session;
use crate::Result;
use std::ffi::CString;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Material {
    pub(crate) session: Session,
    pub(crate) info: HAPI_MaterialInfo,
}

impl Material {
    #[inline]
    pub fn node(&self) -> NodeHandle {
        NodeHandle(self.info.nodeId, ())
    }

    #[inline]
    pub fn has_changed(&self) -> bool {
        self.info.hasChanged > 0
    }

    pub fn render_texture(&self, parm_name: &str) -> Result<()> {
        let name = CString::new(parm_name)?;
        let id = crate::ffi::get_parm_id_from_name(&name, self.node(), &self.session)?;
        crate::ffi::render_texture_to_image(&self.session, self.node(), ParmHandle(id, ()))
    }

    pub fn extract_image_to_file(
        &self,
        image_planes: &str,
        path: impl AsRef<Path>,
    ) -> Result<String> {
        let path = path.as_ref();
        let format = CString::new(
            path.extension()
                .expect("extension")
                .to_string_lossy()
                .to_string()
                .to_uppercase(),
        )?;
        let image_planes = CString::new(image_planes)?;
        let dest_folder =
            CString::new(path.parent().expect("parent").to_string_lossy().to_string())?;
        let dest_file = CString::new(
            path.file_stem()
                .expect("extension")
                .to_string_lossy()
                .to_string(),
        )?;
        crate::ffi::extract_image_to_file(
            &self.session,
            self.node(),
            &format,
            &image_planes,
            &dest_folder,
            &dest_file,
        )
    }

    pub fn extract_image_to_memory(&self, image_planes: &str, format: &str) -> Result<Vec<i8>> {
        let format = CString::new(format)?;
        let image_planes = CString::new(image_planes)?;
        crate::ffi::extract_image_to_memory(&self.session, self.node(), &format, &image_planes)
    }

    pub fn set_image_info(&self, info: &ImageInfo) -> Result<()> {
        crate::ffi::set_image_info(&self.session, self.node(), info)
    }

    pub fn get_image_info(&self) -> Result<ImageInfo> {
        crate::ffi::get_image_info(&self.session, self.node()).map(|inner| ImageInfo { inner })
    }

    pub fn get_image_planes(&self) -> Result<Vec<String>> {
        crate::ffi::get_image_planes(&self.session, self.node()).map(|a| a.into_iter().collect())
    }
}

pub fn get_supported_image_formats(session: &Session) -> Result<Vec<ImageFileFormat<'_>>> {
    crate::ffi::get_supported_image_file_formats(session).map(|v| {
        v.into_iter()
            .map(|inner| ImageFileFormat { inner, session })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Materials;
    use crate::session::tests::with_session;

    #[test]
    fn image_file_formats() {
        with_session(|session| {
            let formats = get_supported_image_formats(session).unwrap();
            assert!(formats
                .iter()
                .find(|f| f.name().unwrap() == "JPEG")
                .is_some());
            assert!(formats
                .iter()
                .find(|f| f.extension().unwrap() == "jpg")
                .is_some());
        });
    }

    #[test]
    fn extract_image() {
        with_session(|session| {
            let node = session.create_node("Object/spaceship", None, None).unwrap();
            node.cook(None).unwrap();
            let geo = node.geometry().expect("geometry").unwrap();
            let mats = geo.get_materials(None).expect("materials");
            if let Some(Materials::Single(mat)) = mats {
                let mut info = mat.get_image_info().unwrap();
                info.set_x_res(512);
                info.set_y_res(512);
                mat.render_texture("baseColorMap").unwrap();
                mat.set_image_info(&info).unwrap();
                let ip = mat.get_image_planes().unwrap();
                assert!(ip.iter().find(|ip| *ip == "C").is_some());
                let file = std::env::temp_dir().join("hapi.jpeg");
                mat.extract_image_to_file("C", file).expect("extract_image");
                mat.render_texture("baseColorMap").unwrap();
                let bytes = mat
                    .extract_image_to_memory("C", "JPEG")
                    .expect("extract_image");
                assert!(bytes.len() > 0);
            } else {
                unreachable!();
            }
        });
    }
}
