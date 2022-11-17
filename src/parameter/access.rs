use super::*;

use std::ffi::CString;

pub use crate::{
    ffi::enums::{ChoiceListType, ParmType},
    ffi::ParmInfo,
};

use crate::errors::Result;

impl IntParameter {
    /// Set parameter value at index.
    pub fn set(&self, index: i32, value: i32) -> Result<()> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::set_parm_int_value(self.wrap.node, session, &name, index, value)
    }

    /// Get parameter value at index.
    pub fn get(&self, index: i32) -> Result<i32> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::get_parm_int_value(self.wrap.node, session, &name, index)
    }

    /// Set all parameter tuple values
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

    /// Set parameter tuple values
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
    /// Set parameter value at index.
    pub fn set(&self, index: i32, value: f32) -> Result<()> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::set_parm_float_value(self.wrap.node, session, &name, index, value)
    }

    /// Get parameter value at index.
    pub fn get(&self, index: i32) -> Result<f32> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::get_parm_float_value(self.wrap.node, session, &name, index)
    }

    /// Set all parameter tuple values
    pub fn set_array(&self, values: impl AsRef<[f32]>) -> Result<()> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let mut size = self.wrap.info.size() as usize;
        let values = values.as_ref();
        match values.len() {
            len if len > size => {
                log::warn!("Array length is greater than parm length: {size}");
                size = values.len().min(size);
            }
            len if len == 0 => {
                log::warn!("Parameter::set_array got empty array");
                return Ok(());
            }
            _ => {}
        }
        crate::ffi::set_parm_float_values(
            self.wrap.node,
            session,
            self.wrap.info.float_values_index(),
            size as i32,
            values.as_ref(),
        )
    }

    /// Get all parameter tuple values
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
    /// Set parameter value at index.
    pub fn set(&self, index: i32, value: impl AsRef<str>) -> Result<()> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let value = CString::new(value.as_ref())?;
        crate::ffi::set_parm_string_value(
            self.wrap.node,
            session,
            self.wrap.info.id(),
            index,
            &value,
        )
    }

    /// Get parameter value at index.
    pub fn get(&self, index: i32) -> Result<String> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::get_parm_string_value(self.wrap.node, session, &name, index)
    }
    /// Set all parameter tuple values
    pub fn set_array<T: AsRef<str>>(&self, val: impl AsRef<[T]>) -> Result<()> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        let values = val
            .as_ref()
            .iter()
            .map(|s| CString::new(s.as_ref()))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        crate::ffi::set_parm_string_values(self.wrap.node, session, self.wrap.info.id(), &values)
    }

    /// Get all parameter tuple values
    pub fn get_array(&self) -> Result<Vec<String>> {
        let session = &self.wrap.info.session;
        debug_assert!(self.wrap.node.is_valid(session)?);
        crate::ffi::get_parm_string_values(
            self.wrap.node,
            session,
            self.wrap.info.string_values_index(),
            self.wrap.info.size(),
        )
        .map(|array| array.into())
    }
}
