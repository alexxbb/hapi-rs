use crate::auto::bindings as ffi;
use crate::session::Session;
use crate::{get_string, Result};
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AssetLibrary {
    lib_id: ffi::HAPI_AssetLibraryId,
    session: Session,
}

impl AssetLibrary {
    pub fn from_file(file: &str, session: Session) -> Result<AssetLibrary> {
        unsafe {
            let mut lib_id = MaybeUninit::uninit();
            let cs = CString::from_vec_unchecked(Vec::from(file));
            ffi::HAPI_LoadAssetLibraryFromFile(
                session.ptr(),
                cs.as_ptr(),
                true as i8,
                lib_id.as_mut_ptr(),
            )
            .result(session.ptr(), None)?;
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
            .result(self.session.ptr(), None)?;
            Ok(num_assets.assume_init())
        }
    }

    pub fn get_asset_names(&self) -> Result<Vec<String>> {
        let num_assets = self.get_asset_count()? as usize;
        let names = unsafe {
            let mut names = -1;
            let _r = ffi::HAPI_GetAvailableAssets(
                self.session.ptr(),
                self.lib_id,
                &mut names as *mut _,
                1,
            );
            std::slice::from_raw_parts(&names as *const _, num_assets)
        };
        names
            .iter()
            .map(|i| get_string(*i, self.session.ptr()))
            .collect::<Result<Vec<_>>>()
    }
}
