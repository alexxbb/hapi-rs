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
//!     assert_eq!(p.get(0).unwrap(), "hello");
//!     assert!(p.set(0, "world").is_ok());
//! }
//! ```
//! Extra parameter features are available in [`ParmBaseTrait`]

mod base;
mod access;
pub use crate::ffi::enums::ParmType;
pub use crate::ffi::structs::{KeyFrame, ParmInfo};
use crate::node::{HoudiniNode, NodeHandle};
use crate::Result;
pub use base::*;

#[derive(Debug, Clone, Copy, PartialEq)]
/// An internal handle to a parameter
pub struct ParmHandle(pub crate::ffi::raw::HAPI_ParmId);

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
        let wrap = ParmInfoWrap { info, node };
        match wrap.info.parm_type() {
            ParmType::Int | ParmType::Toggle | ParmType::Multiparmlist => {
                Parameter::Int(IntParameter(wrap))
            }
            ParmType::Button => Parameter::Button(IntParameter(wrap)),
            ParmType::Float | ParmType::Color => Parameter::Float(FloatParameter(wrap)),
            ParmType::String
            | ParmType::Node
            | ParmType::PathFile
            | ParmType::PathFileDir
            | ParmType::PathFileGeo
            | ParmType::PathFileImage => Parameter::String(StringParameter(wrap)),
            _ => Parameter::Other(BaseParameter(wrap)),
        }
    }
    /// Information about the parameter
    #[inline]
    pub fn info(&self) -> &ParmInfo {
        &self.base().info
    }

    /// Parameter internal name
    #[inline]
    pub fn name(&self) -> Result<String> {
        self.info().name()
    }

    /// Parameter UI label
    #[inline]
    pub fn label(&self) -> Result<String> {
        self.info().label()
    }

    /// Number or elements in the parameter
    #[inline]
    pub fn size(&self) -> i32 {
        self.info().size()
    }

    /// Parameter parent if any (examples are multi-parm or Folder type parameters)
    pub fn parent(&self) -> Result<Option<ParmInfo>> {
        let wrap = self.base();
        debug_assert!(wrap.info.session.is_valid());
        match wrap.info.parent_id() {
            ParmHandle(-1) => Ok(None),
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

    pub(crate) fn base(&self) -> &ParmInfoWrap {
        match self {
            Parameter::Float(p) => &p.0,
            Parameter::Int(p) => &p.0,
            Parameter::Button(p) => &p.0,
            Parameter::String(p) => &p.0,
            Parameter::Other(p) => &p.0,
        }
    }
}

impl ParmHandle {
    /// Find a parameter handle by name
    pub fn from_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        debug_assert!(node.is_valid()?);
        let name = std::ffi::CString::new(name)?;
        let id = crate::ffi::get_parm_id_from_name(&name, node.handle, &node.session)?;
        Ok(ParmHandle(id))
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
    pub(crate) fn from_parm_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        debug_assert!(node.is_valid()?);
        let name = std::ffi::CString::new(name)?;
        let info = crate::ffi::get_parm_info_from_name(node.handle, &node.session, &name);
        info.map(|info| ParmInfo {
            inner: info,
            session: node.session.clone(),
            name: Some(name),
        })
    }

    pub(crate) fn from_parm_handle(handle: ParmHandle, node: &HoudiniNode) -> Result<Self> {
        let parm_info = crate::ffi::get_parm_info(node.handle, &node.session, handle)?;
        Ok(ParmInfo {
            inner: parm_info,
            session: node.session.clone(),
            name: None,
        })
    }
}
