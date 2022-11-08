//! Reading and setting node parameters, setting expressions and keyframes.
//!
//! Different parameter types are modeled with a [Parameter] enum, to get to a concrete type,
//! use pattern matching:
//!
//! ```
//! use hapi_rs::session::new_in_process;
//! use hapi_rs::parameter::*;
//! let session = new_in_process(None).unwrap();
//! let lib = session.load_asset_file("otls/hapi_parms.hda").unwrap();
//! let node = lib.try_create_first().unwrap();
//! if let Parameter::String(p) = node.parameter("single_string").unwrap() {
//!     assert_eq!(p.get_value().unwrap(), &["hello"]);
//!     assert!(p.set_value(&["world".to_string()]).is_ok());
//! }
//! ```

#[cfg(test)]
mod tests;
mod base;
mod access;
use std::ffi::{CStr, CString};
pub use base::*;
use crate::ffi::enums::ParmType;
use crate::Result;
use crate::node::{HoudiniNode, NodeHandle};
use crate::ffi::structs::ParmInfo;


#[derive(Debug, Clone, Copy)]
/// An internal handle to a parameter
pub struct ParmHandle(pub crate::ffi::raw::HAPI_ParmId, pub(crate) ());

#[derive(Debug)]
/// Enum of different parameter types.
pub enum Parameter {
    /// `ParmType::Float | ParmType::Color`
    Float(FloatParameter),
    /// `ParmType::Int | ParmType::Toggle`
    Int(IntParameter),
    /// `ParmType::String | ParmType::Node | ParmType::PathFile | ParmType::PathFileDir | ParmType::PathFileGeo | ParmType::PathFileImage`
    String(StringParameter),
    /// `ParmType::Int`
    Button(IntParameter),
    /// `Other ParmType::_`
    Other(BaseParameter),
}

impl Parameter {
    pub(crate) fn new(node: NodeHandle, info: ParmInfo) -> Parameter {
        let base = ParmNodeWrap { info, node };
        match base.info.parm_type() {
            ParmType::Int | ParmType::Toggle | ParmType::Multiparmlist => {
                Parameter::Int(IntParameter { wrap: base })
            }
            ParmType::Button => Parameter::Button(IntParameter { wrap: base }),
            ParmType::Float | ParmType::Color => Parameter::Float(FloatParameter { wrap: base }),
            ParmType::String
            | ParmType::Node
            | ParmType::PathFile
            | ParmType::PathFileDir
            | ParmType::PathFileGeo
            | ParmType::PathFileImage => Parameter::String(StringParameter { wrap: base }),
            _ => Parameter::Other(BaseParameter { wrap: base }),
        }
    }
    /// Information about the parameter
    #[inline]
    pub fn info(&self) -> &ParmInfo {
        &self.base().info
    }

    /// Parameter name
    #[inline]
    pub fn name(&self) -> Result<String> {
        self.info().name()
    }

    /// Parameter label
    #[inline]
    pub fn label(&self) -> Result<String> {
        self.info().label()
    }

    /// Parameter parent if any (examples are mutli-parm or Folder type parameters)
    pub fn parent(&self) -> Result<Option<ParmInfo>> {
        let wrap = self.base();
        debug_assert!(wrap.info.session.is_valid());
        match wrap.info.parent_id() {
            ParmHandle(-1, ()) => Ok(None),
            handle => {
                let session = wrap.info.session.clone();
                let info = crate::ffi::get_parm_info(wrap.node, &session, handle)?;
                Ok(Some(ParmInfo {
                    inner: info,
                    session,
                    name: None,
                }))
            }
        }
    }

    pub(crate) fn base(&self) -> &ParmNodeWrap {
        match self {
            Parameter::Float(p) => &p.wrap,
            Parameter::Int(p) => &p.wrap,
            Parameter::Button(p) => &p.wrap,
            Parameter::String(p) => &p.wrap,
            Parameter::Other(p) => &p.wrap,
        }
    }
}

impl ParmHandle {
    /// Find a parameter handle by name
    pub fn from_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        debug_assert!(node.is_valid()?);
        let name = CString::new(name)?;
        let id = crate::ffi::get_parm_id_from_name(&name, node.handle, &node.session)?;
        Ok(ParmHandle(id, ()))
    }
    /// Retrieve parameter information from of the handle
    pub fn info(&self, node: &HoudiniNode) -> Result<ParmInfo> {
        debug_assert!(node.is_valid()?);
        let info = crate::ffi::get_parm_info(node.handle, &node.session, *self)?;
        Ok(ParmInfo {
            inner: info,
            session: node.session.clone(),
            name: None,
        })
    }
}

impl ParmInfo {
    pub fn from_parm_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        debug_assert!(node.is_valid()?);
        let name = CString::new(name)?;
        let info = crate::ffi::get_parm_info_from_name(node.handle, &node.session, &name);
        info.map(|info| ParmInfo {
            inner: info,
            session: node.session.clone(),
            name: Some(name)
        })
    }

    pub fn into_node_parm(self, node: NodeHandle) -> Parameter {
        Parameter::new(node, self)
    }
}
