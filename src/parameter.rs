//! Reading and setting node parameters, setting expressions and keyframes.
//!
//! Different parameter types are modeled with a [Parameter] enum, to get to a concrete type,
//! use pattern matching:
//!
//! ```ignore
//! let node: HoudiniNode;
//! if let Parameter:String(path) = node.parameter("file") {
//!     path.get_value();
//!     path.set_value(&["/path/to/file"]);
//! }
//! ```
use std::ffi::CString;

use log::warn;

pub use crate::{
    ffi::enums::{ChoiceListType, ParmType},
    ffi::ParmInfo,
};

use crate::{
    errors::Result,
    ffi::{KeyFrame, ParmChoiceInfo},
    node::{HoudiniNode, NodeHandle},
};

/// Common trait for parameters
pub trait ParmBaseTrait {
    type ValueType;

    #[inline]
    fn name(&self) -> Result<String> {
        self.wrap().info.name()
    }
    #[inline]
    fn is_menu(&self) -> bool {
        !matches!(self.wrap().info.choice_list_type(), ChoiceListType::None)
    }
    #[doc(hidden)]
    fn wrap(&self) -> &ParmNodeWrap;
    /// If parameter is a menu type, return a vec of menu items
    fn menu_items(&self) -> Result<Option<Vec<ParmChoiceInfo>>> {
        if !self.is_menu() {
            return Ok(None);
        }
        let wrap = self.wrap();
        debug_assert!(wrap.info.session.is_valid());
        let parms = crate::ffi::get_parm_choice_list(
            wrap.node,
            &wrap.info.session,
            wrap.info.choice_index(),
            wrap.info.choice_count(),
        );
        let parms = parms.map(|v| {
            v.into_iter()
                .map(|p| ParmChoiceInfo {
                    inner: p,
                    session: wrap.info.session.clone(),
                })
                .collect::<Vec<ParmChoiceInfo>>()
        })?;
        Ok(Some(parms))
    }
    /// Returns a parameter expression string
    fn expression(&self, index: i32) -> Result<Option<String>> {
        let wrap = self.wrap();
        debug_assert!(wrap.info.session.is_valid());
        let name = wrap.info.name_cstr()?;
        crate::ffi::get_parm_expression(wrap.node, &wrap.info.session, &name, index)
    }

    /// Checks if parameter has an expression
    fn has_expression(&self, index: i32) -> Result<bool> {
        let wrap = self.wrap();
        debug_assert!(wrap.info.session.is_valid());
        let name = wrap.info.name_cstr()?;
        crate::ffi::parm_has_expression(wrap.node, &wrap.info.session, &name, index)
    }

    /// Set parameter expression
    fn set_expression(&self, value: &str, index: i32) -> Result<()> {
        let wrap = self.wrap();
        debug_assert!(wrap.info.session.is_valid());
        let value = CString::new(value)?;
        crate::ffi::set_parm_expression(
            wrap.node,
            &wrap.info.session,
            wrap.info.id(),
            &value,
            index,
        )
    }

    /// Get parameter value
    fn get_value(&self) -> Result<Vec<Self::ValueType>>;
    /// Set parameter value
    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>;

    /// Set keyframes on the parameter
    fn set_anim_curve(&self, index: i32, keys: &[KeyFrame]) -> Result<()> {
        let wrap = self.wrap();
        debug_assert!(wrap.info.session.is_valid());
        let keys =
            unsafe { std::mem::transmute::<&[KeyFrame], &[crate::ffi::raw::HAPI_Keyframe]>(keys) };
        crate::ffi::set_parm_anim_curve(&wrap.info.session, wrap.node, wrap.info.id(), index, keys)
    }
}

#[derive(Debug, Clone, Copy)]
/// An internal handle to a parameter
pub struct ParmHandle(pub crate::ffi::raw::HAPI_ParmId, pub(crate) ());

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
        })
    }

    pub fn into_node_parm(self, node: NodeHandle) -> Parameter {
        Parameter::new(node, self)
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct ParmNodeWrap {
    pub(crate) info: ParmInfo,
    pub(crate) node: NodeHandle,
}

#[derive(Debug)]
#[doc(hidden)]
pub struct BaseParameter {
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub struct FloatParameter {
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub struct IntParameter {
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub struct StringParameter {
    pub(crate) wrap: ParmNodeWrap,
}

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

impl ParmBaseTrait for FloatParameter {
    type ValueType = f32;

    #[doc(hidden)]
    fn wrap(&self) -> &ParmNodeWrap {
        &self.wrap
    }

    fn get_value(&self) -> Result<Vec<Self::ValueType>> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        crate::ffi::get_parm_float_values(
            self.wrap.node,
            session,
            self.wrap.info.float_values_index(),
            self.wrap.info.size(),
        )
    }

    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>,
    {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        crate::ffi::set_parm_float_values(
            self.wrap.node,
            session,
            self.wrap.info.float_values_index(),
            self.wrap.info.size(),
            val.as_ref(),
        )
    }
}

impl ParmBaseTrait for IntParameter {
    type ValueType = i32;

    #[doc(hidden)]
    fn wrap(&self) -> &ParmNodeWrap {
        &self.wrap
    }

    fn get_value(&self) -> Result<Vec<Self::ValueType>> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        crate::ffi::get_parm_int_values(
            self.wrap.node,
            session,
            self.wrap.info.int_values_index(),
            self.wrap.info.size(),
        )
    }

    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>,
    {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        crate::ffi::set_parm_int_values(
            self.wrap.node,
            session,
            self.wrap.info.int_values_index(),
            self.wrap.info.size(),
            val.as_ref(),
        )
    }
}

impl IntParameter {
    /// Emulates a button press action
    pub fn press_button(&self) -> Result<()> {
        if !matches!(self.wrap.info.parm_type(), ParmType::Button) {
            warn!("Parm {} not a Button type", self.wrap.info.name()?);
        }
        self.set_value([1])
    }
}

impl ParmBaseTrait for StringParameter {
    type ValueType = String;

    #[doc(hidden)]
    fn wrap(&self) -> &ParmNodeWrap {
        &self.wrap
    }

    fn get_value(&self) -> Result<Vec<String>> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        Ok(crate::ffi::get_parm_string_values(
            self.wrap.node,
            session,
            self.wrap.info.string_values_index(),
            self.wrap.info.size(),
        )?
        .into_iter()
        .collect::<Vec<_>>())
    }

    // TODO Maybe take it out of the trait? AsRef makes it an extra String copy. Consider ToOwned?
    // What a hell did I mean by that?
    // Update: 2 month later still can't remember
    // Update: 3 month later. Still no clue, moving on for now
    // Update: 1 year past. Meh, maybe later
    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>,
    {
        let c_str: std::result::Result<Vec<CString>, _> = val
            .as_ref()
            .iter()
            .map(|s| CString::new(s.clone()))
            .collect();
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        crate::ffi::set_parm_string_values(self.wrap.node, session, &self.wrap.info.id(), &c_str?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::tests::with_session;

    #[test]
    fn node_parameters() {
        with_session(|session| {
            let node = session
                .create_node("Object/hapi_parms", None, None)
                .expect("create_node");
            for p in node.parameters().unwrap() {
                assert!(p.name().is_ok());
            }
            if let Parameter::Float(p) = node.parameter("color").unwrap() {
                let val = p.get_value().unwrap();
                assert_eq!(&val, &[0.55f32, 0.75, 0.95]);
                p.set_value([0.7, 0.5, 0.3]).unwrap();
                let val = p.get_value().unwrap();
                assert_eq!(&val, &[0.7f32, 0.5, 0.3]);
            }
            if let Parameter::Float(p) = node.parameter("single_float").unwrap() {
                p.set_expression("$T", 0).unwrap();
                assert_eq!("$T", p.expression(0).unwrap().unwrap());
            }

            if let Parameter::String(p) = node.parameter("multi_string").unwrap() {
                let mut value = p.get_value().unwrap();
                assert_eq!(vec!["foo 1", "bar 2", "baz 3"], value);
                value[0] = "cheese".to_owned();
                p.set_value(value).unwrap();
                assert_eq!("cheese", p.get_value().unwrap()[0]);
            }

            if let Parameter::Int(p) = node.parameter("ord_menu").unwrap() {
                assert!(p.is_menu());
                assert_eq!(p.get_value().unwrap()[0], 0);
                if let Some(items) = p.menu_items().unwrap() {
                    assert_eq!(items[0].value().unwrap(), "foo");
                    assert_eq!(items[0].label().unwrap(), "Foo");
                }
            }

            if let Parameter::Int(p) = node.parameter("toggle").unwrap() {
                assert_eq!(p.get_value().unwrap()[0], 0);
                p.set_value([1]).unwrap();
                assert_eq!(p.get_value().unwrap()[0], 1);
            }

            // test button callback
            if let Parameter::Button(ip) = node.parameter("button").unwrap() {
                if let Parameter::String(sp) = node.parameter("single_string").unwrap() {
                    assert_eq!(sp.get_value().unwrap()[0], "hello");
                    ip.press_button().unwrap();
                    assert_eq!(sp.get_value().unwrap()[0], "set from callback");
                }
            }
        });
    }

    #[test]
    fn set_anim_curve() {
        use crate::ffi::KeyFrame;

        with_session(|session| {
            let node = session
                .create_node("Object/null", "set_anim_curve", None)
                .unwrap();

            if let Ok(Parameter::Float(p)) = node.parameter("scale") {
                let keys = vec![
                    KeyFrame {
                        time: 0.0,
                        value: 0.0,
                        in_tangent: 0.0,
                        out_tangent: 0.0,
                    },
                    KeyFrame {
                        time: 1.0,
                        value: 1.0,
                        in_tangent: 0.0,
                        out_tangent: 0.0,
                    },
                ];

                p.set_anim_curve(0, &keys).expect("set_anim_curve")
            }
        });
    }
}
