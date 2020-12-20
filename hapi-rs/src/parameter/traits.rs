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
                    Ok(Cow::Borrowed(std::str::from_utf8_unchecked(s.as_bytes())))
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
    fn get_value(&self) -> Result<Vec<Self::ValueType>>;
    fn set_value<T>(&self, val: T) -> Result<()>
        where T: AsRef<[Self::ValueType]>;
}
