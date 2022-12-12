use crate::ffi::enums::ChoiceListType;
use crate::ffi::{KeyFrame, ParmChoiceInfo, ParmInfo};
use crate::node::NodeHandle;
use crate::session::Session;
use crate::Result;
use std::borrow::Cow;
use std::ffi::{CStr, CString};

/// Common trait for parameters
pub trait ParmBaseTrait {
    #[inline]
    fn name(&self) -> Result<String> {
        self.inner().info.name()
    }

    #[inline]
    fn session(&self) -> &Session {
        &self.inner().info.session
    }

    #[inline]
    fn node(&self) -> NodeHandle {
        self.inner().node
    }

    #[inline]
    fn size(&self) -> i32 {
        self.inner().info.size()
    }

    /// If the parameter has choice menu.
    #[inline]
    fn is_menu(&self) -> bool {
        !matches!(self.inner().info.choice_list_type(), ChoiceListType::None)
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
                    session: inner.info.session.clone(),
                })
                .collect::<Vec<ParmChoiceInfo>>()
        })?;
        Ok(Some(parms))
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
        let wrap = self.inner();
        debug_assert!(wrap.info.session.is_valid());
        let name = self.c_name()?;
        crate::ffi::parm_has_expression(wrap.node, &wrap.info.session, &name, index)
    }

    /// Set parameter expression
    fn set_expression(&self, value: &str, index: i32) -> Result<()> {
        let wrap = self.inner();
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

    /// Set keyframes on the parameter
    fn set_anim_curve(&self, index: i32, keys: &[KeyFrame]) -> Result<()> {
        let wrap = self.inner();
        debug_assert!(wrap.info.session.is_valid());
        let keys =
            unsafe { std::mem::transmute::<&[KeyFrame], &[crate::ffi::raw::HAPI_Keyframe]>(keys) };
        crate::ffi::set_parm_anim_curve(&wrap.info.session, wrap.node, wrap.info.id(), index, keys)
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
