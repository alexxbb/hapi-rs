use std::ffi::{CStr, CString};
use std::borrow::Cow;
use crate::ffi::{KeyFrame, ParmInfo, ParmChoiceInfo};
use crate::ffi::enums::ChoiceListType;
use crate::node::NodeHandle;
use crate::Result;

/// Common trait for parameters
pub trait ParmBaseTrait {

    #[inline]
    fn name(&self) -> Result<String> {
        self.wrap().info.name()
    }

    #[doc(hidden)]
    // If the parameter was obtained by name (node.parameter(..))
    // we store the name in the info struct, otherwise, call API to get name
    fn c_name(&self) -> Result<Cow<CStr>> {
        let wrap = self.wrap();
        match wrap.info.name.as_deref() {
            None => wrap.info.name_cstr().map(Cow::Owned),
            Some(name) => Ok(Cow::Borrowed(name))
        }
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

    /// Set keyframes on the parameter
    fn set_anim_curve(&self, index: i32, keys: &[KeyFrame]) -> Result<()> {
        let wrap = self.wrap();
        debug_assert!(wrap.info.session.is_valid());
        let keys =
            unsafe { std::mem::transmute::<&[KeyFrame], &[crate::ffi::raw::HAPI_Keyframe]>(keys) };
        crate::ffi::set_parm_anim_curve(&wrap.info.session, wrap.node, wrap.info.id(), index, keys)
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

impl ParmBaseTrait for FloatParameter {
    #[doc(hidden)]
    fn wrap(&self) -> &ParmNodeWrap {
        &self.wrap
    }
}

impl ParmBaseTrait for IntParameter {
    #[doc(hidden)]
    fn wrap(&self) -> &ParmNodeWrap {
        &self.wrap
    }

}

impl ParmBaseTrait for StringParameter {
    #[doc(hidden)]
    fn wrap(&self) -> &ParmNodeWrap {
        &self.wrap
    }
}
