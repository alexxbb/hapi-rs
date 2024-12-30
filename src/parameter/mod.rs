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
use crate::node::{HoudiniNode, NodeHandle, Session};
use crate::Result;
pub use base::*;
use std::fmt::Debug;

/// An internal handle to a parameter
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    /// A convenient method to evaluate a parameter value as string for debugging.
    pub fn value_as_debug(&self) -> Result<Box<dyn Debug>> {
        match self {
            Parameter::Float(parm) => Ok(Box::new(parm.get_array()?)),
            Parameter::Int(parm) | Parameter::Button(parm) => Ok(Box::new(parm.get_array()?)),
            Parameter::String(parm) => Ok(Box::new(parm.get_array()?)),
            Parameter::Other(parm) => Ok(Box::new(parm.0.info.parm_type())),
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
        debug_assert!(wrap.info.1.is_valid());
        match wrap.info.parent_id() {
            ParmHandle(-1) => Ok(None),
            handle => {
                let session = wrap.info.1.clone();
                let info = crate::ffi::get_parm_info(wrap.node, &session, handle)?;
                Ok(Some(ParmInfo(info, session, None)))
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

impl ParmBaseTrait for Parameter {
    fn inner(&self) -> &ParmInfoWrap {
        match self {
            Parameter::Float(p) => &p.0,
            Parameter::Int(p) => &p.0,
            Parameter::Button(p) => &p.0,
            Parameter::String(p) => &p.0,
            Parameter::Other(p) => &p.0,
        }
    }

    fn inner_mut(&mut self) -> &mut ParmInfoWrap {
        match self {
            Parameter::Float(p) => &mut p.0,
            Parameter::Int(p) => &mut p.0,
            Parameter::Button(p) => &mut p.0,
            Parameter::String(p) => &mut p.0,
            Parameter::Other(p) => &mut p.0,
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
        Ok(ParmInfo::new(info, node.session.clone(), None))
    }
}

impl ParmInfo {
    pub(crate) fn from_parm_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        debug_assert!(node.is_valid()?);
        let name = std::ffi::CString::new(name)?;
        let info = crate::ffi::get_parm_info_from_name(node.handle, &node.session, &name);
        info.map(|info| ParmInfo::new(info, node.session.clone(), Some(name)))
    }

    pub(crate) fn from_parm_handle(
        parm: ParmHandle,
        node: NodeHandle,
        session: &Session,
    ) -> Result<Self> {
        let parm_info = crate::ffi::get_parm_info(node, session, parm)?;
        Ok(ParmInfo::new(parm_info, session.clone(), None))
    }
}
