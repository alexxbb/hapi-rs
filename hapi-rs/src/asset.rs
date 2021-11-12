use std::ffi::CString;

use log::debug;

use crate::ffi::raw as ffi;
use crate::ffi::raw::{ChoiceListType, ParmType};
use crate::{
    errors::{HapiError, Kind, Result},
    ffi::{AssetInfo, ParmInfo},
    node::HoudiniNode,
    parameter::ParmChoiceInfo,
    session::Session,
};

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

pub struct AssetParmIter<'a> {
    iter: std::slice::Iter<'a, ParmInfo>,
    values: &'a AssetParmValues,
}

impl<'a> Iterator for AssetParmIter<'a> {
    type Item = AssetParm<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|info| AssetParm {
            info,
            values: &self.values,
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
        &self.info
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

    /// Returns the name of first asset in the library
    pub fn get_first_name(&self) -> Result<String> {
        match self.get_asset_names()?.first() {
            Some(name) => Ok(name.clone()),
            None => Err(HapiError::new(
                Kind::Other("Empty AssetLibrary".to_string()),
                None,
                None,
            )),
        }
    }

    /// Try to create the first available asset in the library
    pub fn try_create_first(&self) -> Result<HoudiniNode> {
        self.session
            .create_node_blocking(&self.get_first_name()?, None, None)
    }

    pub fn get_asset_parms(&self, asset: Option<&str>) -> Result<AssetParameters> {
        let mut _name;
        let asset_name = if let Some(name) = asset {
            name
        } else {
            _name = self.get_first_name()?;
            _name.as_str()
        };
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
    use crate::session::tests::{with_session, OTLS};

    fn _load_asset(name: &str, ses: &super::Session) -> super::AssetLibrary {
        let otl = OTLS.get(name).expect("otl not found");
        ses.load_asset_file(otl).expect("load_asset_file")
    }

    #[test]
    fn get_asset_count() {
        with_session(|session| {
            let lib = _load_asset("parameters", &session);
            assert_eq!(lib.get_asset_count().expect("get_asset_count"), 1);
        });
    }

    #[test]
    fn get_asset_names() {
        with_session(|session| {
            let lib = _load_asset("parameters", &session);
            assert!(lib
                .get_asset_names()
                .expect("get_asset_name")
                .contains(&"Object/hapi_parms".to_string()));
        });
    }

    #[test]
    fn get_first_name() {
        with_session(|session| {
            let lib = _load_asset("parameters", &session);
            assert_eq!(lib.get_first_name(), Ok(String::from("Object/hapi_parms")));
        });
    }

    #[test]
    #[ignore]
    fn asset_parameters() {
        // TODO: This is crashing the server
        // with_session(|session| {
        //     assert!(session.is_valid());
        //     let otl = OTLS.get("parameters").unwrap();
        //     let lib = session
        //         .load_asset_file(otl)
        //         .expect(&format!("Could not load {}", otl));
        //     let _ = lib.get_asset_parms(Some("Object/hapi_parms"));
        //     // assert!(parms.is_ok());
        // });
    }
}
