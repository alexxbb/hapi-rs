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
    pub file: Option<PathBuf>,
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
            file: Some(file),
        })
    }

    /// Load asset library from memory
    pub fn from_memory(session: Session, data: &[u8]) -> Result<AssetLibrary> {
        debug!("Loading library from memory");
        debug_assert!(session.is_valid());
        let data: &[i8] = unsafe { std::mem::transmute(data) };
        let lib_id = crate::ffi::load_library_from_memory(&session, data, true)?;
        Ok(AssetLibrary {
            lib_id,
            session,
            file: None,
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

    /// Create a node for an asset. This function is a convenient form of [`Session::create_node`]
    /// in a way that it makes sure that a correct parent network node is also created for
    /// assets other than Object level such as Cop, Top, etc.
    pub fn create_asset_for_node<T: AsRef<str>>(
        &self,
        name: T,
        label: Option<T>,
    ) -> Result<HoudiniNode> {
        // Most common HDAs are Object/asset and Sop/asset which HAPI can create directly in /obj,
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
        // There's no root network manager for Sop node types.
        let (manager, subnet) = if context == "Sop" {
            (None, None)
        } else {
            let manager_type = context.parse::<ManagerType>()?;
            let subnet = match manager_type {
                ManagerType::Cop => Some("img"),
                ManagerType::Chop => Some("ch"),
                ManagerType::Top => Some("topnet"),
                _ => None,
            };
            (Some(manager_type), subnet)
        };

        // If subnet is Some, we get the manager node for this context and use it as parent.
        let parent = match subnet {
            Some(subnet) => {
                // manager is always Some if subnet is Some
                let parent = self.session.get_manager_node(manager.unwrap())?;
                Some(
                    self.session
                        .create_node_with(subnet, parent.handle, None, false)?
                        .handle,
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
            .create_node_with(full_name, parent, label.as_ref().map(|v| v.as_ref()), false)
    }

    /// Try to create the first found asset in the library.
    /// This is a convenience function for:
    /// ```
    /// use hapi_rs::session::{new_in_process};
    /// let session = new_in_process(None).unwrap();
    /// let lib = session.load_asset_file("otls/hapi_geo.hda").unwrap();
    /// let names = lib.get_asset_names().unwrap();
    /// session.create_node(&names[0]).unwrap();
    /// ```
    /// Except that it also handles non Object level assets, e.g. Cop network HDA.
    pub fn try_create_first(&self) -> Result<HoudiniNode> {
        debug_assert!(self.session.is_valid());
        let name = self
            .get_first_name()?
            .ok_or_else(|| crate::HapiError::internal("Library file is empty"))?;
        self.create_asset_for_node(name, None)
    }

    /// Returns a struct holding the asset parameter information and values
    pub fn get_asset_parms(&self, asset: impl AsRef<str>) -> Result<AssetParameters> {
        debug_assert!(self.session.is_valid());
        // let _lock = self.session.lock();
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
        .map(|info| ParmInfo::new(info, self.session.clone(), None));
        let values =
            crate::ffi::get_asset_def_parm_values(self.lib_id, &asset_name, &self.session, &count)?;
        let menus = values.3.into_iter().map(|info| ParmChoiceInfo {
            inner: info,
            session: self.session.clone().into(),
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
