use crate::auto::bindings as ffi;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::path::Path;
use log::debug;

use crate::{
    node::{HoudiniNode, NodeHandle},
    session::Session,
    errors::Result,
    stringhandle::*,
};

#[derive(Debug, Clone)]
pub struct AssetLibrary {
    lib_id: ffi::HAPI_AssetLibraryId,
    session: Session,
}

/// https://github.com/sideeffects/HoudiniEngineForUnity/blob/5b2d34bd5a04513288f4991048bf9c5ecceacac5/Plugins/HoudiniEngineUnity/Scripts/Asset/HEU_HoudiniAsset.cs#L1995
impl AssetLibrary {
    pub fn from_file(session: Session, file: impl AsRef<str>) -> Result<AssetLibrary> {
        debug!("Loading library: {}", file.as_ref());
        let cs = CString::new(file.as_ref())?;
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

// TODO: AssetDefinition

#[derive(Debug)]
pub struct AssetInfo<'session> {
    pub(crate) inner: ffi::HAPI_AssetInfo,
    session: &'session Session,
}

macro_rules! _get_str {
    ($m:ident->$f:ident) => {
        pub fn $m(&self) -> Result<String> {
            self.session.get_string(self.inner.$f)
        }
    };
}

impl<'node> AssetInfo<'node> {
    pub fn new(node: &'node HoudiniNode) -> Result<AssetInfo<'_>> {
        let info = unsafe {
            let mut info = MaybeUninit::uninit();
            ffi::HAPI_GetAssetInfo(node.session.ptr(), node.handle.0, info.as_mut_ptr())
                .result_with_session(|| node.session.clone())?;
            info.assume_init()
        };

        Ok(AssetInfo {
            inner: info,
            session: &node.session,
        })
    }
    get!(node_id->nodeId->[handle: NodeHandle]);
    get!(object_node_id->objectNodeId->[handle: NodeHandle]);
    get!(has_ever_cooked->hasEverCooked->bool);
    get!(have_objects_changed->haveObjectsChanged->bool);
    get!(have_materials_changed->haveMaterialsChanged->bool);
    get!(object_count->objectCount->i32);
    get!(handle_count->handleCount->i32);
    get!(transform_input_count->transformInputCount->i32);
    get!(geo_input_count->geoInputCount->i32);
    get!(geo_output_count->geoOutputCount->i32);

    _get_str!(name->nameSH);
    _get_str!(label->labelSH);
    _get_str!(file_path->filePathSH);
    _get_str!(version->versionSH);
    _get_str!(full_op_name->fullOpNameSH);
    _get_str!(help_text->helpTextSH);
    _get_str!(help_url->helpURLSH);

}

