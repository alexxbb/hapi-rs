use crate::ffi::enums::ChoiceListType;
use crate::ffi::{KeyFrame, ParmChoiceInfo, ParmInfo};
use crate::node::{NodeHandle, ParmType};
use crate::session::Session;
use crate::Result;
use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::ops::Deref;

use super::Parameter;

/// Common trait for parameters
pub trait ParmBaseTrait {
    #[inline]
    fn name(&self) -> Result<String> {
        self.info().name()
    }

    #[inline]
    fn session(&self) -> &Session {
        &self.info().session
    }

    #[inline]
    fn node(&self) -> NodeHandle {
        self.inner().node
    }

    #[inline]
    fn size(&self) -> i32 {
        self.info().size()
    }

    #[inline]
    fn info(&self) -> &ParmInfo {
        &self.inner().info
    }

    /// If the parameter has choice menu.
    #[inline]
    fn is_menu(&self) -> bool {
        !matches!(self.info().choice_list_type(), ChoiceListType::None)
    }
    /// If parameter is a menu type, return a vec of menu items
    fn menu_items(&self) -> Result<Option<Vec<ParmChoiceInfo>>> {
        if !self.is_menu() {
            return Ok(None);
        }
        let inner = self.inner();
        debug_assert!(inner.info.session.is_valid());
        let parms = crate::ffi::get_parm_choice_list(
            inner.node,
            &inner.info.session,
            inner.info.choice_index(),
            inner.info.choice_count(),
        );
        let parms = parms.map(|v| {
            v.into_iter()
                .map(|p| ParmChoiceInfo {
                    inner: p,
                    session: inner.info.session.deref().clone(),
                })
                .collect::<Vec<ParmChoiceInfo>>()
        })?;
        Ok(Some(parms))
    }

    /// If the parameter is a multiparm, return its children parms.
    /// NOTE: THis is not a recommended way to traverse parameters in general,
    /// this is here for convenience only
    fn multiparm_children(&self) -> Result<Option<Vec<Parameter>>> {
        let inner = self.inner();
        if inner.info.parm_type() != ParmType::Multiparmlist {
            return Ok(None);
        }
        let node = inner.node.to_node(&inner.info.session)?;
        let mut all_parameters = node.parameters()?;
        all_parameters.retain(|parm| {
            parm.info().is_child_of_multi_parm() && parm.info().parent_id() == inner.info.id()
        });

        Ok(Some(all_parameters))
    }

    /// Returns a parameter expression string
    fn expression(&self, index: i32) -> Result<Option<String>> {
        let inner = self.inner();
        debug_assert!(inner.info.session.is_valid());
        let name = self.c_name()?;
        crate::ffi::get_parm_expression(inner.node, &inner.info.session, &name, index)
    }

    /// Checks if parameter has an expression
    fn has_expression(&self, index: i32) -> Result<bool> {
        let inner = self.inner();
        debug_assert!(inner.info.session.is_valid());
        let name = self.c_name()?;
        crate::ffi::parm_has_expression(inner.node, &inner.info.session, &name, index)
    }

    /// Set parameter expression
    fn set_expression(&self, value: &str, index: i32) -> Result<()> {
        let inner = self.inner();
        debug_assert!(inner.info.session.is_valid());
        let value = CString::new(value)?;
        crate::ffi::set_parm_expression(
            inner.node,
            &inner.info.session,
            inner.info.id(),
            &value,
            index,
        )
    }

    /// Remove parameter expression
    fn remove_expression(&self, index: i32) -> Result<()> {
        let inner = self.inner();
        debug_assert!(inner.info.session.is_valid());
        crate::ffi::remove_parm_expression(inner.node, &inner.info.session, inner.info.id(), index)
    }

    /// Revert parameter at index to its default value
    fn revert_to_default(&self, index: i32) -> Result<()> {
        let inner = self.inner();
        crate::ffi::revert_parameter_to_default(
            inner.node,
            &inner.info.session,
            &self.c_name()?,
            index,
        )
    }

    /// Set keyframes on the parameter
    fn set_anim_curve(&self, index: i32, keys: &[KeyFrame]) -> Result<()> {
        let inner = self.inner();
        debug_assert!(inner.info.session.is_valid());
        let keys =
            unsafe { std::mem::transmute::<&[KeyFrame], &[crate::ffi::raw::HAPI_Keyframe]>(keys) };
        crate::ffi::set_parm_anim_curve(
            &inner.info.session,
            inner.node,
            inner.info.id(),
            index,
            keys,
        )
    }

    #[doc(hidden)]
    // If the parameter was obtained by name (node.parameter(..))
    // we store the name in the info struct, otherwise, call API to get name
    fn c_name(&self) -> Result<Cow<CStr>> {
        let inner = self.inner();
        match inner.info.name.as_deref() {
            None => inner.info.name_cstr().map(Cow::Owned),
            Some(name) => Ok(Cow::Borrowed(name)),
        }
    }
    #[doc(hidden)]
    fn inner(&self) -> &ParmInfoWrap;
}

#[derive(Debug)]
#[doc(hidden)]
pub struct ParmInfoWrap {
    pub(crate) info: ParmInfo,
    pub(crate) node: NodeHandle,
}

#[derive(Debug)]
#[doc(hidden)]
pub struct BaseParameter(pub(crate) ParmInfoWrap);

/// Represents float parameters, including `Color` type.
#[derive(Debug)]
pub struct FloatParameter(pub(crate) ParmInfoWrap);

/// Represents integer parameters, including `Button` type
#[derive(Debug)]
pub struct IntParameter(pub(crate) ParmInfoWrap);

/// Represents string parameters of many different types.
#[derive(Debug)]
pub struct StringParameter(pub(crate) ParmInfoWrap);

impl ParmBaseTrait for FloatParameter {
    #[inline]
    #[doc(hidden)]
    fn inner(&self) -> &ParmInfoWrap {
        &self.0
    }
}

impl ParmBaseTrait for IntParameter {
    #[inline]
    #[doc(hidden)]
    fn inner(&self) -> &ParmInfoWrap {
        &self.0
    }
}

impl ParmBaseTrait for StringParameter {
    #[inline]
    #[doc(hidden)]
    fn inner(&self) -> &ParmInfoWrap {
        &self.0
    }
}
