//! For loading digital assets and reading their parameters.
//! [Documentation](https://www.sidefx.com/docs/hengine/_h_a_p_i__assets.html)
use crate::ffi::raw as ffi;
use crate::ffi::raw::{ChoiceListType, ParmType};
use crate::node::ManagerType;
use crate::{
    errors::Result, ffi::ParmChoiceInfo, ffi::ParmInfo, node::HoudiniNode, session::Session,
    HapiError,
};
use log::debug;
use std::ffi::CString;
use std::path::PathBuf;

struct AssetParmValues {
    int: Vec<i32>,
    float: Vec<f32>,
    string: Vec<String>,
    menus: Vec<ParmChoiceInfo>,
}

/// Holds asset parameters data.
/// Call `into_iter` to get an iterator over each parameter
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
    /// Find asset parameter by name
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

/// Iterator over asset parameter default values
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

/// Holds info and default value of a parameter
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

/// Parameter default value
#[derive(Debug)]
pub enum ParmValue<'a> {
    Int(&'a [i32]),
    Float(&'a [f32]),
    String(&'a [String]),
    Toggle(bool),
    NoDefault,
}

impl<'a> AssetParm<'a> {
    /// Get parameter default value
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
            Float | Color => {
                let start = self.info.float_values_index() as usize;
                ParmValue::Float(&self.values.float[start..start + size])
            }
            String | PathFileGeo | PathFile | PathFileImage | PathFileDir | Node => {
                let start = self.info.string_values_index() as usize;
                ParmValue::String(&self.values.string[start..start + size])
            }
            _ => ParmValue::NoDefault,
        }
    }

    /// Returns menu parameter items.
    /// Note, dynamic(script) menus should be queried directly from a node.
    pub fn menu_items(&self) -> Option<&[ParmChoiceInfo]> {
        if let ChoiceListType::None = self.choice_list_type() {
            return None;
        }
        let count = self.info.choice_count() as usize;
        let start = self.info.choice_index() as usize;
        Some(&self.values.menus[start..start + count])
    }
}

/// A handle to a loaded HDA file
#[derive(Debug, Clone)]
pub struct AssetLibrary {
    lib_id: ffi::HAPI_AssetLibraryId,
    session: Session,
    pub file: PathBuf,
}

impl AssetLibrary {
    /// Load an asset from file
    pub fn from_file(session: Session, file: impl AsRef<std::path::Path>) -> Result<AssetLibrary> {
        let file = file.as_ref().to_path_buf();
        debug!("Loading library file: {:?}", file);
        debug_assert!(session.is_valid());
        let cs = CString::new(file.as_os_str().to_string_lossy().to_string())?;
        let lib_id = crate::ffi::load_library_from_file(&cs, &session, true)?;
        Ok(AssetLibrary {
            lib_id,
            session,
            file,
        })
    }

    /// Get number of assets defined in the current library
    pub fn get_asset_count(&self) -> Result<i32> {
        debug_assert!(self.session.is_valid());
        crate::ffi::get_asset_count(self.lib_id, &self.session)
    }

    /// Get asset names this library contains
    pub fn get_asset_names(&self) -> Result<Vec<String>> {
        debug_assert!(self.session.is_valid());
        debug!("Retrieving asset names from: {:?}", self.file);
        let num_assets = self.get_asset_count()?;
        crate::ffi::get_asset_names(self.lib_id, num_assets, &self.session)
            .map(|a| a.into_iter().collect())
    }

    /// Returns the name of first asset in the library
    pub fn get_first_name(&self) -> Result<Option<String>> {
        debug_assert!(self.session.is_valid());
        self.get_asset_names().map(|names| names.first().cloned())
    }

    /// Instantiate a node. This function is more convenient than [`Session::create_node`]
    /// as it makes sure that a correct parent network node is also created.
    pub fn create_node(&self, name: impl AsRef<str>) -> Result<HoudiniNode> {
        // Most common HDAs are Object/asset which HAPI can create directly in /obj,
        // but for some assets type like Cop, Top a manager node must be created first
        debug!("Trying to create a node for operator: {}", name.as_ref());
        let Some((context, operator)) = name.as_ref().split_once('/') else {
            return Err(HapiError::internal("Node name must be fully qualified"))
        };
        // Strip operator namespace if present
        let context = if let Some((_, context)) = context.split_once("::") {
            context
        } else {
            context
        };
        let context: ManagerType = context.parse()?;
        let subnet = match context {
            ManagerType::Cop => Some("img"),
            ManagerType::Chop => Some("ch"),
            ManagerType::Top => Some("topnet"),
            _ => None,
        };

        // If subnet is Some, we get the manager node for this context and use it as parent.
        let parent = match subnet {
            Some(subnet) => {
                let manager = self.session.get_manager_node(context)?;
                Some(
                    self.session
                        .create_node(subnet, None, Some(manager.handle))?,
                )
            }
            None => None,
        };
        // If passing a parent, operator name must be stripped of the context name
        let full_name = if parent.is_some() {
            operator
        } else {
            name.as_ref()
        };
        self.session
            .create_node(full_name, None, parent.map(|n| n.handle))
    }

    /// Try to create the first found asset in the library.
    /// This is a convenience function for:
    /// ```
    /// use hapi_rs::session::{new_in_process};
    /// let session = new_in_process(None).unwrap();
    /// let lib = session.load_asset_file("otls/hapi_geo.hda").unwrap();
    /// let names = lib.get_asset_names().unwrap();
    /// session.create_node(&names[0], None, None).unwrap();
    /// ```
    /// Except that it also handles non Object level assets, e.g. Cop network HDA.
    pub fn try_create_first(&self) -> Result<HoudiniNode> {
        debug_assert!(self.session.is_valid());
        let name = self
            .get_first_name()?
            .ok_or_else(|| crate::HapiError::internal("Library file is empty"))?;
        self.create_node(name)
    }

    /// Returns a struct holding the asset parameter information and values
    pub fn get_asset_parms(&self, asset: impl AsRef<str>) -> Result<AssetParameters> {
        debug_assert!(self.session.is_valid());
        let _lock = self.session.lock();
        log::debug!("Reading asset parameter list of {}", asset.as_ref());
        let asset_name = CString::new(asset.as_ref())?;
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
            name: None,
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

#[cfg(test)]
mod tests {
    use super::AssetLibrary;
    use crate::session::tests::with_session;
    use once_cell::sync::OnceCell;

    fn _parms_asset(session: &super::Session) -> &'static AssetLibrary {
        static _LIB: OnceCell<AssetLibrary> = OnceCell::new();
        _LIB.get_or_init(|| {
            session
                .load_asset_file("otls/hapi_parms.hda")
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
            // Script Menus are not evaluated from asset definition, only from a node instance
            let parm = parms.find_parameter("script_menu").expect("parm");
            assert!(parm.menu_items().expect("Script Items").is_empty());
        });
    }
}
