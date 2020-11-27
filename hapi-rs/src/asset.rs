use crate::auto::bindings as ffi;
use crate::session::Session;
use crate::{stringhandle::*, Result};
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::path::Path;
use log::debug;

#[derive(Debug, Clone)]
pub struct AssetLibrary {
    lib_id: ffi::HAPI_AssetLibraryId,
    session: Session,
}

impl AssetLibrary {
    pub fn from_file(session: Session, file: impl AsRef<std::path::Path>) -> Result<AssetLibrary> {
        let path = file.as_ref().to_string_lossy();
        debug!("Loading library: {}", &path);
        let cs = CString::new(path.as_bytes().to_vec())?;
        unsafe {
            let mut lib_id = MaybeUninit::uninit();
            ffi::HAPI_LoadAssetLibraryFromFile(
                session.ptr(),
                cs.as_ptr(),
                true as i8,
                lib_id.as_mut_ptr(),
            )
            .result_with_session(|| session.clone())?;
            let lib_id = lib_id.assume_init();
            Ok(AssetLibrary { lib_id, session })
        }
    }

    pub fn get_asset_count(&self) -> Result<i32> {
        unsafe {
            let mut num_assets = MaybeUninit::uninit();
            ffi::HAPI_GetAvailableAssetCount(
                self.session.ptr(),
                self.lib_id,
                num_assets.as_mut_ptr(),
            )
            .result_with_session(|| self.session.clone())?;
            Ok(num_assets.assume_init())
        }
    }

    pub fn get_asset_names(&self) -> Result<Vec<String>> {
        let num_assets = self.get_asset_count()?;
        let handles = unsafe {
            let mut names = vec![0; num_assets as usize];
            ffi::HAPI_GetAvailableAssets(
                self.session.ptr(),
                self.lib_id,
                names.as_mut_ptr(),
                num_assets,
            )
            .result_with_session(|| self.session.clone())?;
            names
        };

        get_string_batch(&handles, &self.session)
    }
}
