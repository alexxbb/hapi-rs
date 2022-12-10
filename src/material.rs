//! Rendering material textures to memory or disk
//!
use crate::ffi::{raw::HAPI_MaterialInfo, ImageInfo};
use crate::node::{HoudiniNode, NodeHandle};
use crate::parameter::ParmHandle;
use crate::session::Session;
use crate::Result;
use std::ffi::CString;
use std::path::Path;

#[derive(Debug, Clone)]
/// Represents a material node (SHOP) with methods for texture baking
pub struct Material {
    pub(crate) session: Session,
    pub(crate) info: HAPI_MaterialInfo,
}

impl Material {
    #[inline]
    pub fn node(&self) -> Result<HoudiniNode> {
        HoudiniNode::new(self.session.clone(), self.node_handle(), None)
    }

    #[inline]
    fn node_handle(&self) -> NodeHandle {
        NodeHandle(self.info.nodeId)
    }

    #[inline]
    pub fn has_changed(&self) -> bool {
        self.info.hasChanged > 0
    }

    pub fn render_texture(&self, parm_name: &str) -> Result<()> {
        debug_assert!(self.session.is_valid());
        let name = CString::new(parm_name)?;
        let id = crate::ffi::get_parm_id_from_name(&name, self.node_handle(), &self.session)?;
        crate::ffi::render_texture_to_image(&self.session, self.node_handle(), ParmHandle(id))
    }

    pub fn extract_image_to_file(
        &self,
        image_planes: impl AsRef<str>,
        path: impl AsRef<Path>,
    ) -> Result<String> {
        debug_assert!(self.session.is_valid());
        extract_image_to_file(&self.session, self.node_handle(), image_planes, path)
    }

    pub fn extract_image_to_memory(
        &self,
        buffer: &mut Vec<u8>,
        image_planes: impl AsRef<str>,
        format: impl AsRef<str>,
    ) -> Result<()> {
        debug_assert!(self.session.is_valid());
        extract_image_to_memory(
            &self.session,
            self.node_handle(),
            buffer,
            image_planes,
            format,
        )
    }

    pub fn set_image_info(&self, info: &ImageInfo) -> Result<()> {
        debug_assert!(self.session.is_valid());
        crate::ffi::set_image_info(&self.session, self.node_handle(), info)
    }

    pub fn get_image_info(&self) -> Result<ImageInfo> {
        debug_assert!(self.session.is_valid());
        crate::ffi::get_image_info(&self.session, self.node_handle())
            .map(|inner| ImageInfo { inner })
    }

    pub fn get_image_planes(&self) -> Result<Vec<String>> {
        debug_assert!(self.session.is_valid());
        crate::ffi::get_image_planes(&self.session, self.node_handle())
            .map(|a| a.into_iter().collect())
    }
}

pub(crate) fn extract_image_to_file(
    session: &Session,
    node: NodeHandle,
    image_planes: impl AsRef<str>,
    path: impl AsRef<Path>,
) -> Result<String> {
    debug_assert!(session.is_valid());
    let path = path.as_ref();
    let format = CString::new(
        path.extension()
            .expect("extension")
            .to_string_lossy()
            .to_string()
            .to_uppercase(),
    )?;
    let image_planes = CString::new(image_planes.as_ref())?;
    let dest_folder = CString::new(path.parent().expect("parent").to_string_lossy().to_string())?;
    let dest_file = CString::new(
        path.file_stem()
            .expect("extension")
            .to_string_lossy()
            .to_string(),
    )?;
    crate::ffi::extract_image_to_file(
        session,
        node,
        &format,
        &image_planes,
        &dest_folder,
        &dest_file,
    )
}

pub(crate) fn extract_image_to_memory(
    session: &Session,
    node: NodeHandle,
    buffer: &mut Vec<u8>,
    image_planes: impl AsRef<str>,
    format: impl AsRef<str>,
) -> Result<()> {
    let format = CString::new(format.as_ref())?;
    let image_planes = CString::new(image_planes.as_ref())?;
    crate::ffi::extract_image_to_memory(session, node, buffer, &format, &image_planes)
}

#[cfg(test)]
mod tests {
    use crate::geometry::Materials;
    use crate::session::tests::with_session;

    #[test]
    fn image_file_formats() {
        with_session(|session| {
            let formats = session.get_supported_image_formats().unwrap();
            assert!(formats.iter().any(|f| f.name().unwrap() == "JPEG"));
            assert!(formats.iter().any(|f| f.extension().unwrap() == "jpg"));
        });
    }

    #[test]
    fn extract_image() {
        with_session(|session| {
            let node = session.create_node("Object/spaceship", None, None).unwrap();
            node.cook().unwrap();
            let geo = node.geometry().expect("geometry").unwrap();
            let mats = geo.get_materials(None).expect("materials");
            if let Some(Materials::Single(mat)) = mats {
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
                unreachable!();
            }
        });
    }
}
