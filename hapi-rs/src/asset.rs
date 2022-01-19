use crate::ffi::raw as ffi;
use crate::ffi::raw::{ChoiceListType, ParmType};
use crate::{
    errors::Result,
    ffi::ParmChoiceInfo,
    ffi::{AssetInfo, ParmInfo},
    node::HoudiniNode,
    session::Session,
};
use log::debug;
use std::ffi::CString;

struct AssetParmValues {
    int: Vec<i32>,
    float: Vec<f32>,
    string: Vec<String>,
    menus: Vec<ParmChoiceInfo>,
}

pub struct AssetParameters {
    infos: Vec<ParmInfo>,
    values: AssetParmValues,
}

impl<'a> IntoIterator for &'a AssetParameters {
    type Item = AssetParm<'a>;
    type IntoIter = AssetParmIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AssetParmIter {
            iter: self.infos.iter(),
            values: &self.values,
        }
    }
}

impl AssetParameters {
    pub fn find_parameter(&self, name: &str) -> Option<AssetParm<'_>> {
        self.infos
            .iter()
            .find(|p| p.name().unwrap() == name)
            .map(|info| AssetParm {
                info,
                values: &self.values,
            })
    }
}

pub struct AssetParmIter<'a> {
    iter: std::slice::Iter<'a, ParmInfo>,
    values: &'a AssetParmValues,
}

impl<'a> Iterator for AssetParmIter<'a> {
    type Item = AssetParm<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|info| AssetParm {
            info,
            values: self.values,
        })
    }
}

pub struct AssetParm<'a> {
    info: &'a ParmInfo,
    values: &'a AssetParmValues,
}

impl<'a> std::ops::Deref for AssetParm<'a> {
    type Target = ParmInfo;

    fn deref(&self) -> &Self::Target {
        self.info
    }
}

#[derive(Debug)]
pub enum ParmValue<'a> {
    Int(&'a [i32]),
    Float(&'a [f32]),
    String(&'a [String]),
    Toggle(bool),
    Other(String),
}

impl<'a> AssetParm<'a> {
    pub fn default_value(&self) -> ParmValue<'a> {
        let size = self.info.size() as usize;
        use ParmType::*;
        match self.info.parm_type() {
            Int | Button => {
                let start = self.info.int_values_index() as usize;
                ParmValue::Int(&self.values.int[start..start + size])
            }
            Toggle => {
                let start = self.info.int_values_index() as usize;
                ParmValue::Toggle(self.values.int[start] == 1)
            }
            Float => {
                let start = self.info.float_values_index() as usize;
                ParmValue::Float(&self.values.float[start..start + size])
            }
            String | PathFileGeo | PathFile | PathFileImage | PathFileDir => {
                let start = self.info.string_values_index() as usize;
                ParmValue::String(&self.values.string[start..start + size])
            }
            _ => ParmValue::Other(format!("TODO: {:?}", self.info.parm_type())),
        }
    }

    pub fn menu_items(&self) -> Option<&[ParmChoiceInfo]> {
        if let ChoiceListType::None = self.choice_list_type() {
            return None;
        }
        let count = self.info.choice_count() as usize;
        let start = self.info.choice_index() as usize;
        Some(&self.values.menus[start..start + count])
    }
}

#[derive(Debug, Clone)]
pub struct AssetLibrary {
    lib_id: ffi::HAPI_AssetLibraryId,
    session: Session,
}

impl AssetLibrary {
    pub fn from_file(session: Session, file: impl AsRef<str>) -> Result<AssetLibrary> {
        debug!("Loading library: {}", file.as_ref());
        debug_assert!(session.is_valid());
        let cs = CString::new(file.as_ref())?;
        let lib_id = crate::ffi::load_library_from_file(&cs, &session, true)?;
        Ok(AssetLibrary { lib_id, session })
    }

    pub fn get_asset_count(&self) -> Result<i32> {
        debug_assert!(self.session.is_valid());
        crate::ffi::get_asset_count(self.lib_id, &self.session)
    }

    pub fn get_asset_names(&self) -> Result<Vec<String>> {
        debug_assert!(self.session.is_valid());
        let num_assets = self.get_asset_count()?;
        crate::ffi::get_asset_names(self.lib_id, num_assets, &self.session)
            .map(|a| a.into_iter().collect())
    }

    /// Returns the name of first asset in the library
    pub fn get_first_name(&self) -> Result<Option<String>> {
        debug_assert!(self.session.is_valid());
        self.get_asset_names().map(|names| names.first().cloned())
    }

    /// Try to create the first available asset in the library
    pub fn try_create_first(&self) -> Result<HoudiniNode> {
        debug_assert!(self.session.is_valid());
        let name = self
            .get_first_name()?
            .ok_or_else(|| crate::errors::HapiError {
                kind: crate::errors::Kind::Other("Library is empty".to_string()),
                message: None,
                session: None,
            })?;
        self.session.create_node_blocking(&name, None, None)
    }

    pub fn get_asset_parms(&self, asset: impl AsRef<str>) -> Result<AssetParameters> {
        debug_assert!(self.session.is_valid());
        let _lock = self.session.handle.1.lock();
        let asset_name = String::from(asset.as_ref());
        log::debug!("Reading asset parameter list of {asset_name}");
        let asset_name = CString::new(asset_name)?;
        let count = crate::ffi::get_asset_def_parm_count(self.lib_id, &asset_name, &self.session)?;
        let infos = crate::ffi::get_asset_def_parm_info(
            self.lib_id,
            &asset_name,
            count.parm_count,
            &self.session,
        )?
        .into_iter()
        .map(|info| ParmInfo {
            inner: info,
            session: self.session.clone(),
        });
        let values =
            crate::ffi::get_asset_def_parm_values(self.lib_id, &asset_name, &self.session, &count)?;
        let menus = values.3.into_iter().map(|info| ParmChoiceInfo {
            inner: info,
            session: self.session.clone(),
        });
        let values = AssetParmValues {
            int: values.0,
            float: values.1,
            string: values.2,
            menus: menus.collect(),
        };
        Ok(AssetParameters {
            infos: infos.collect(),
            values,
        })
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

#[cfg(test)]
mod tests {
    use super::AssetLibrary;
    use crate::session::tests::with_session;
    use once_cell::sync::OnceCell;

    fn _parms_asset(session: &super::Session) -> &'static AssetLibrary {
        static _LIB: OnceCell<AssetLibrary> = OnceCell::new();
        _LIB.get_or_init(|| {
            session
                .load_asset_file("../otls/hapi_parms.hda")
                .expect("load_asset_file")
        })
    }

    #[test]
    fn get_asset_count() {
        with_session(|session| {
            let lib = _parms_asset(session);
            assert_eq!(lib.get_asset_count().expect("get_asset_count"), 1);
        });
    }

    #[test]
    fn get_asset_names() {
        with_session(|session| {
            let lib = _parms_asset(session);
            assert!(lib
                .get_asset_names()
                .expect("get_asset_name")
                .contains(&"Object/hapi_parms".to_string()));
        });
    }

    #[test]
    fn get_first_name() {
        with_session(|session| {
            let lib = _parms_asset(session);
            assert_eq!(
                lib.get_first_name(),
                Ok(Some(String::from("Object/hapi_parms")))
            );
        });
    }

    #[test]
    fn asset_parameters() {
        use super::ParmValue;
        with_session(|session| {
            let lib = _parms_asset(session);
            let parms = lib.get_asset_parms("Object/hapi_parms").unwrap();

            let parm = parms.find_parameter("single_string").expect("parm");
            if let ParmValue::String([val]) = parm.default_value() {
                assert_eq!(val, "hello");
            } else {
                panic!("parm is not a string");
            }
            let parm = parms.find_parameter("float3").expect("parm");
            if let ParmValue::Float(val) = parm.default_value() {
                assert_eq!(val, &[0.1, 0.2, 0.3]);
            } else {
                panic!("parm is not a float3");
            }
        });
    }

    #[test]
    fn asset_menu_parameters() {
        with_session(|session| {
            let lib = _parms_asset(session);
            let parms = lib.get_asset_parms("Object/hapi_parms").unwrap();

            let parm = parms.find_parameter("string_menu").expect("parm");
            let menu_values: Vec<_> = parm
                .menu_items()
                .expect("Menu items")
                .iter()
                .map(|p| p.value().unwrap())
                .collect();
            assert_eq!(menu_values, &["item_1", "item_2", "item_3"]);
        });
    }
}
