use super::*;

use std::ffi::{CStr, CString};

pub use crate::{
    ffi::enums::{ChoiceListType, ParmType},
    ffi::ParmInfo,
};

use crate::{errors::Result, ffi::{KeyFrame, ParmChoiceInfo}, HapiError, node::{HoudiniNode, NodeHandle}};
use crate::stringhandle::StringArray;


impl IntParameter {
    pub fn set(&self, index: i32, value: i32) -> Result<()> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::set_parm_int_value(
            self.wrap.node,
            session,
            &name,
            index,
            value
        )
    }

    pub fn get(&self, index: i32) -> Result<i32> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::get_parm_int_value(
            self.wrap.node,
            session,
            &name,
            index,
        )
    }

    pub fn set_array(&self, val: impl AsRef<[i32]>) -> Result<()> {
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

    pub fn get_array(&self) -> Result<Vec<i32>> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        crate::ffi::get_parm_int_values(
            self.wrap.node,
            session,
            self.wrap.info.int_values_index(),
            self.wrap.info.size(),
        )
    }


    /// Emulates a button press action
    pub fn press_button(&self) -> Result<()> {
        if !matches!(self.wrap.info.parm_type(), ParmType::Button) {
            log::warn!("Parm {} not a Button type", self.wrap.info.name()?);
        }
        self.set(0, 1)
    }
}

impl FloatParameter {

    pub fn set(&self, index: i32, value: f32) -> Result<()> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::set_parm_float_value(
            self.wrap.node,
            session,
            &name,
            index,
            value
        )
    }

    pub fn get(&self, index: i32) -> Result<f32> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::get_parm_float_value(
            self.wrap.node,
            session,
            &name,
            index,
        )
    }

    pub fn set_array(&self, val: impl AsRef<[f32]>) -> Result<()> {
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

    pub fn get_array(&self) -> Result<Vec<f32>> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        crate::ffi::get_parm_float_values(
            self.wrap.node,
            session,
            self.wrap.info.float_values_index(),
            self.wrap.info.size(),
        )
    }

}

impl StringParameter {

    pub fn set(&self, index: i32, value: impl AsRef<str>) -> Result<()> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let value = CString::new(value.as_ref())?;
        crate::ffi::set_parm_string_value(
            self.wrap.node,
            session,
            self.wrap.info.id(),
            index,
            &value
        )
    }

    pub fn get(&self, index: i32) -> Result<String> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::get_parm_string_value(
            self.wrap.node,
            session,
            &name,
            index,
        )
    }
    pub fn set_array<'a, T: AsRef<str>>(&self, val: impl AsRef<[T]>) -> Result<()> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let values = val.as_ref().iter().map(|s|CString::new(s.as_ref())).collect::<std::result::Result<Vec<_>, _>>()?;
        crate::ffi::set_parm_string_values(
            self.wrap.node,
            session,
            self.wrap.info.id(),
            &values,
        )
    }

    pub fn get_array(&self) -> Result<Vec<String>> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        crate::ffi::get_parm_string_values(
            self.wrap.node,
            session,
            self.wrap.info.string_values_index(),
            self.wrap.info.size(),
        ).map(|array|array.into())
    }

}