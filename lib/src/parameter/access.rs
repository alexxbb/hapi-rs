use super::*;

use std::ffi::CString;

pub use crate::ffi::enums::ParmType;

use crate::errors::Result;

impl IntParameter {
    /// Set parameter value at index.
    pub fn set(&self, index: i32, value: i32) -> Result<()> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::set_parm_int_value(self.0.node, session, &name, index, value)
    }

    /// Get parameter value at index.
    pub fn get(&self, index: i32) -> Result<i32> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::get_parm_int_value(self.0.node, session, &name, index)
    }

    /// Set all parameter tuple values
    pub fn set_array(&self, val: impl AsRef<[i32]>) -> Result<()> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        crate::ffi::set_parm_int_values(
            self.0.node,
            session,
            self.0.info.int_values_index(),
            self.0.info.size(),
            val.as_ref(),
        )
    }

    /// Set parameter tuple values
    pub fn get_array(&self) -> Result<Vec<i32>> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        crate::ffi::get_parm_int_values(
            self.0.node,
            session,
            self.0.info.int_values_index(),
            self.0.info.size(),
        )
    }

    /// Emulates a button press action
    pub fn press_button(&self) -> Result<()> {
        if !matches!(self.0.info.parm_type(), ParmType::Button) {
            log::warn!("Parm {} not a Button type", self.0.info.name()?);
        }
        self.set(0, 1)
    }
}

impl FloatParameter {
    /// Set parameter value at index.
    pub fn set(&self, index: i32, value: f32) -> Result<()> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::set_parm_float_value(self.0.node, session, &name, index, value)
    }

    /// Get parameter value at index.
    pub fn get(&self, index: i32) -> Result<f32> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::get_parm_float_value(self.0.node, session, &name, index)
    }

    /// Set all parameter tuple values
    pub fn set_array(&self, values: impl AsRef<[f32]>) -> Result<()> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        let mut size = self.0.info.size() as usize;
        let values = values.as_ref();
        match values.len() {
            len if len > size => {
                log::warn!("Array length is greater than parm length: {size}");
                size = values.len().min(size);
            }
            0 => {
                log::warn!("Parameter::set_array got empty array");
                return Ok(());
            }
            _ => {}
        }
        crate::ffi::set_parm_float_values(
            self.0.node,
            session,
            self.0.info.float_values_index(),
            size as i32,
            values,
        )
    }

    /// Get all parameter tuple values
    pub fn get_array(&self) -> Result<Vec<f32>> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        crate::ffi::get_parm_float_values(
            self.0.node,
            session,
            self.0.info.float_values_index(),
            self.0.info.size(),
        )
    }
}

impl StringParameter {
    /// Set parameter value at index.
    pub fn set(&self, index: i32, value: impl AsRef<str>) -> Result<()> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        let value = CString::new(value.as_ref())?;
        crate::ffi::set_parm_string_value(self.0.node, session, self.0.info.id(), index, &value)
    }

    /// Get parameter value at index.
    pub fn get(&self, index: i32) -> Result<String> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        let name = self.c_name()?;
        crate::ffi::get_parm_string_value(self.0.node, session, &name, index)
    }
    /// Set all parameter tuple values
    pub fn set_array<T: AsRef<str>>(&self, val: impl AsRef<[T]>) -> Result<()> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        let values = val
            .as_ref()
            .iter()
            .map(|s| CString::new(s.as_ref()))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        crate::ffi::set_parm_string_values(self.0.node, session, self.0.info.id(), &values)
    }

    /// Get all parameter tuple values
    pub fn get_array(&self) -> Result<Vec<String>> {
        let session = &self.0.info.1;
        debug_assert!(self.0.node.is_valid(session)?);
        crate::ffi::get_parm_string_values(
            self.0.node,
            session,
            self.0.info.string_values_index(),
            self.0.info.size(),
        )
        .map(|array| array.into_iter().collect())
    }

    /// Save/Download a file referenced in this parameter to a given file.
    /// `filename` must include the desired extension to work properly.
    pub fn save_parm_file(&self, destination_dir: &std::path::Path, filename: &str) -> Result<()> {
        log::debug!(
            "Saving parameter file to: {:?}/{}",
            destination_dir,
            filename
        );
        let dest_dir = crate::utils::path_to_cstring(destination_dir)?;
        let dest_file = CString::new(filename)?;
        crate::ffi::get_file_parm(
            &self.0.info.1,
            self.0.node,
            &self.c_name()?,
            &dest_dir,
            &dest_file,
        )
    }

    /// If parameter is a `ParmType::Node` type, set it to reference another node
    pub fn set_value_as_node(&self, value: impl AsRef<NodeHandle>) -> Result<()> {
        debug_assert!(self.0.node.is_valid(self.session())?);
        if self.0.info.parm_type() == ParmType::Node {
            crate::ffi::set_parm_node_value(
                self.session(),
                self.0.node,
                &self.c_name()?,
                *value.as_ref(),
            )
        } else {
            Ok(())
        }
    }
    /// Return a handle to a node if the parameter is of type `ParmType::Node`
    pub fn get_value_as_node(&self) -> Result<Option<NodeHandle>> {
        debug_assert!(self.0.node.is_valid(self.session())?);
        if self.0.info.parm_type() == ParmType::Node {
            crate::ffi::get_parm_node_value(self.session(), self.0.node, &self.c_name()?)
        } else {
            Ok(None)
        }
    }
}
