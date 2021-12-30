use crate::ffi::ImageFileFormat;
use crate::ffi::{raw::HAPI_MaterialInfo, ImageInfo};
use crate::node::NodeHandle;
use crate::parameter::ParmHandle;
use crate::session::Session;
use crate::Result;

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
        let name = std::ffi::CString::new(parm_name)?;
        let id = crate::ffi::get_parm_id_from_name(&name, self.node(), &self.session)?;
        crate::ffi::render_texture_to_image(&self.session, self.node(), ParmHandle(id, ()))
    }

    pub fn set_image_info(&self, info: &ImageInfo) -> Result<()> {
        crate::ffi::set_image_info(&self.session, self.node(), info)
    }

    pub fn get_image_info(&self) -> Result<ImageInfo> {
        crate::ffi::get_image_info(&self.session, self.node()).map(|inner| ImageInfo { inner })
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
}
