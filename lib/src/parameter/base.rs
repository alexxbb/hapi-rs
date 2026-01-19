use crate::Result;
use crate::ffi::enums::ChoiceListType;
use crate::ffi::{KeyFrame, ParmChoiceInfo, ParmInfo};
use crate::node::{NodeHandle, ParmType};
use crate::session::Session;
use std::borrow::Cow;
use std::ffi::{CStr, CString};

use super::Parameter;

/// Common trait for parameters
pub trait ParmBaseTrait {
    #[inline]
    fn name(&self) -> Result<Cow<'_, str>> {
        let inner = self.inner();
        match inner.info.2.as_ref() {
            None => inner.info.name().map(Cow::Owned),
            Some(c_name) => Ok(c_name.to_string_lossy()),
        }
    }

    #[inline]
    fn session(&self) -> &Session {
        &self.info().1
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

    /// Update the internal parameter metadata if Houdini parameter changed,
    /// for example children added to a multi-parm or the menu was updated.
    fn update(&mut self) -> Result<()> {
        let inner = self.inner_mut();
        let name = inner.info.2.take();
        let mut info = ParmInfo::from_parm_handle(inner.info.id(), inner.node, &inner.info.1)?;
        info.2 = name;
        inner.info = info;
        Ok(())
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
        debug_assert!(inner.info.1.is_valid());
        if inner.info.choice_count() == 0 {
            return Ok(Some(Vec::new()));
        }
        let parms = crate::ffi::get_parm_choice_list(
            inner.node,
            &inner.info.1,
            inner.info.choice_index(),
            inner.info.choice_count(),
        );
        let parms = parms.map(|v| {
            use std::ops::Deref;

            v.into_iter()
                .map(|p| ParmChoiceInfo(p, inner.info.1.deref().clone()))
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
        let node = inner.node.to_node(&inner.info.1)?;
        let mut all_parameters = node.parameters()?;
        all_parameters.retain(|parm| {
            parm.info().is_child_of_multi_parm() && parm.info().parent_id() == inner.info.id()
        });

        Ok(Some(all_parameters))
    }

    fn insert_multiparm_instance(&self, position: i32) -> Result<()> {
        let ParmInfoWrap { info, node } = self.inner();
        crate::ffi::insert_multiparm_instance(&info.1, *node, info.id(), position)
    }

    fn remove_multiparm_instance(&self, position: i32) -> Result<()> {
        let ParmInfoWrap { info, node } = self.inner();
        crate::ffi::remove_multiparm_instance(&info.1, *node, info.id(), position)
    }

    /// Returns a parameter expression string
    fn expression(&self, index: i32) -> Result<Option<String>> {
        let inner = self.inner();
        debug_assert!(inner.info.1.is_valid());
        let name = self.c_name()?;
        let expr_string = crate::ffi::get_parm_expression(inner.node, &inner.info.1, &name, index)?;
        Ok(if expr_string.is_empty() {
            None
        } else {
            Some(expr_string)
        })
    }

    /// Checks if parameter has an expression
    fn has_expression(&self, index: i32) -> Result<bool> {
        let inner = self.inner();
        debug_assert!(inner.info.1.is_valid());
        let name = self.c_name()?;
        crate::ffi::parm_has_expression(inner.node, &inner.info.1, &name, index)
    }

    /// Set parameter expression
    fn set_expression(&self, value: &str, index: i32) -> Result<()> {
        let inner = self.inner();
        debug_assert!(inner.info.1.is_valid());
        let value = CString::new(value)?;
        crate::ffi::set_parm_expression(inner.node, &inner.info.1, inner.info.id(), &value, index)
    }

    /// Remove parameter expression
    fn remove_expression(&self, index: i32) -> Result<()> {
        let inner = self.inner();
        debug_assert!(inner.info.1.is_valid());
        crate::ffi::remove_parm_expression(inner.node, &inner.info.1, inner.info.id(), index)
    }

    /// Revert parameter at index to its default value. If `index` is None - reset all instances.
    fn revert_to_default(&self, index: Option<i32>) -> Result<()> {
        let inner = self.inner();
        crate::ffi::revert_parameter_to_default(inner.node, &inner.info.1, &self.c_name()?, index)
    }

    /// Set keyframes on the parameter
    fn set_anim_curve(&self, index: i32, keys: &[KeyFrame]) -> Result<()> {
        let inner = self.inner();
        debug_assert!(inner.info.1.is_valid());
        // SAFETY: Both structures have the same memory layout.
        let keys =
            unsafe { std::mem::transmute::<&[KeyFrame], &[crate::ffi::raw::HAPI_Keyframe]>(keys) };
        crate::ffi::set_parm_anim_curve(&inner.info.1, inner.node, inner.info.id(), index, keys)
    }

    fn has_tag(&self, tag: &str) -> Result<bool> {
        let inner = self.inner();
        let tag = CString::new(tag)?;
        crate::ffi::parm_has_tag(&inner.info.1, inner.node, inner.info.id(), &tag)
    }

    /// Get parameter tag name by index. The number of tags is stored in `self.info().tag_count()`
    fn get_tag_name(&self, tag_index: i32) -> Result<String> {
        let inner = self.inner();
        crate::ffi::get_parm_tag_name(&inner.info.1, inner.node, inner.info.id(), tag_index)
    }

    fn get_tag_value(&self, tag_name: &str) -> Result<String> {
        let inner = self.inner();
        let tag = CString::new(tag_name)?;
        crate::ffi::get_parm_tag_value(&inner.info.1, inner.node, inner.info.id(), &tag)
    }

    #[doc(hidden)]
    // If the parameter was obtained by name (node.parameter(..))
    // we store the name in the info struct, otherwise, call API to get name
    fn c_name(&self) -> Result<Cow<'_, CStr>> {
        let inner = self.inner();
        match inner.info.2.as_deref() {
            None => inner.info.name_cstr().map(Cow::Owned),
            Some(name) => Ok(Cow::Borrowed(name)),
        }
    }
    #[doc(hidden)]
    fn inner(&self) -> &ParmInfoWrap;

    #[doc(hidden)]
    fn inner_mut(&mut self) -> &mut ParmInfoWrap;
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

    #[inline]
    #[doc(hidden)]
    fn inner_mut(&mut self) -> &mut ParmInfoWrap {
        &mut self.0
    }
}

impl ParmBaseTrait for IntParameter {
    #[inline]
    #[doc(hidden)]
    fn inner(&self) -> &ParmInfoWrap {
        &self.0
    }

    #[inline]
    #[doc(hidden)]
    fn inner_mut(&mut self) -> &mut ParmInfoWrap {
        &mut self.0
    }
}

impl ParmBaseTrait for StringParameter {
    #[inline]
    #[doc(hidden)]
    fn inner(&self) -> &ParmInfoWrap {
        &self.0
    }

    #[inline]
    #[doc(hidden)]
    fn inner_mut(&mut self) -> &mut ParmInfoWrap {
        &mut self.0
    }
}
