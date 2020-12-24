use crate::{
    errors::Result,
    ffi,
};
use super::parameter::{
    ParmNodeWrap,
};
use std::borrow::Cow;
use std::ffi::CString;

pub trait ParmBaseTrait<'s> {
    type ValueType;

    fn c_name(&'s self) -> Result<Cow<'s, CString>> {
        let info = &self.wrap().info;
        let n = match info.name.as_ref() {
            None => Ok(Cow::Owned(info.name_cstr()?)),
            Some(n) => Ok(Cow::Borrowed(n)),
        };
        n
    }

    fn name(&'s self) -> Result<Cow<'s, str>> {
        match self.c_name()? {
            Cow::Borrowed(s) => {
                unsafe {
                    let bytes = s.as_bytes();
                    Ok(Cow::Borrowed(std::str::from_utf8_unchecked(&bytes[..bytes.len() - 1])))
                }
            }
            Cow::Owned(s) => {
                Ok(Cow::Owned(s.into_string().unwrap()))
            }
        }
    }
    fn is_menu(&self) -> bool {
        !matches!(self.wrap().info.choice_list_type(), ffi::ChoiceListType::None)
    }
    fn wrap(&self) -> &ParmNodeWrap<'s>;
    fn menu_items(&self) -> Option<Result<Vec<(String, String)>>> {
        if !self.is_menu() {
            return None;
        }
        let wrap = self.wrap();
        Some(
            super::values::get_choice_list(&wrap.node.handle,
                                           &wrap.node.session,
                                           wrap.info.choice_index(),
                                           wrap.info.choice_count())
        )
    }
    fn expression(&'s self, index: i32) -> Result<String> {
        let wrap = self.wrap();
        super::values::get_parm_expression(
            &wrap.node.handle,
            &wrap.node.session,
            self.c_name()?.as_c_str(),
            index)
    }

    fn set_expression(&'s self, value: &str, index: i32) -> Result<()> {
        let wrap = self.wrap();
        let value = CString::new(value)?;
        super::values::set_parm_expression(&wrap.node.handle, &wrap.node.session,
                                           &wrap.info.id(), &value, index)
    }

    fn get_value(&self) -> Result<Vec<Self::ValueType>>;
    fn set_value<T>(&self, val: T) -> Result<()>
        where T: AsRef<[Self::ValueType]>;
}
