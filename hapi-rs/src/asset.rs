use std::ffi::CString;

use log::debug;

use crate::ffi::raw as ffi;
use crate::{
    errors::Result,
    ffi::{AssetInfo, ParmInfo},
    node::HoudiniNode,
    session::Session,
};

#[derive(Debug, Clone)]
pub struct AssetLibrary {
    lib_id: ffi::HAPI_AssetLibraryId,
    session: Session,
}

impl AssetLibrary {
    pub fn from_file(session: Session, file: impl AsRef<str>) -> Result<AssetLibrary> {
        debug!("Loading library: {}", file.as_ref());
        let cs = CString::new(file.as_ref())?;
        let lib_id = crate::ffi::load_library_from_file(&cs, &session, true)?;
        Ok(AssetLibrary { lib_id, session })
    }

    pub fn get_asset_count(&self) -> Result<i32> {
        crate::ffi::get_asset_count(self.lib_id, &self.session)
    }

    pub fn get_asset_names(&self) -> Result<Vec<String>> {
        let num_assets = self.get_asset_count()?;
        crate::ffi::get_asset_names(self.lib_id, num_assets, &self.session)
            .map(|a| a.into_iter().collect())
    }

    pub fn get_asset_parms(&self, asset_name: impl AsRef<str>) -> Result<Vec<ParmInfo>> {
        let cs = CString::new(asset_name.as_ref())?;
        let count = crate::ffi::get_asset_def_parm_count(self.lib_id, &cs, &self.session)?;
        Ok(
            crate::ffi::get_asset_def_parm_info(self.lib_id, &cs, count.parm_count, &self.session)?
                .into_iter()
                .map(|info| ParmInfo {
                    inner: info,
                    session: self.session.clone(),
                    name: None,
                })
                .collect(),
        )
    }
    /// Try to create the first available asset in the library
    pub fn try_create_first(&self) -> Result<HoudiniNode> {
        use crate::errors::{HapiError, Kind};
        match self.get_asset_names()?.first() {
            Some(name) => self.session.create_node_blocking(name, None, None),
            None => Err(HapiError::new(
                Kind::Other("Empty AssetLibrary".to_string()),
                None,
                None,
            )),
        }
    }

    /// Return a vec of parameters of the first available asset in the library
    pub fn try_get_asset_parms(&self) -> Result<Vec<ParmInfo>> {
        use crate::errors::{HapiError, Kind};
        match self.get_asset_names()?.first() {
            Some(name) => self.get_asset_parms(name),
            None => Err(HapiError::new(
                Kind::Other("Empty AssetLibrary".to_string()),
                None,
                None,
            )),
        }
    }
}

impl<'node> AssetInfo<'node> {
    pub fn new(node: &'node HoudiniNode) -> Result<AssetInfo<'_>> {
        Ok(AssetInfo {
            inner: crate::ffi::get_asset_info(node)?,
            session: &node.session,
        })
    }
}
