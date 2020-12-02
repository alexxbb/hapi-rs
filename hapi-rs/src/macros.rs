#[macro_export]
macro_rules! inner_field {
    ($field_name:ident, $func_name:ident, bool) => {
        #[inline]
        pub fn $func_name(&self) -> bool {
            self.inner.$field_name == 1
        }
    };

    ($field_name:ident, $func_name:ident, Result<String>) => {
        #[inline]
        pub fn $func_name(&self) -> Result<String> {
            crate::stringhandle::get_string(self.inner.$field_name, &self.session)
        }
    };
    ($field_name:ident, $func_name:ident, $ret:ty) => {
        #[inline]
        pub fn $func_name(&self) -> $ret {
            self.inner.$field_name
        }
    };
}

#[macro_export]
macro_rules! char_ptr {
    ($lit:expr) => {{
        use std::ffi::CStr;
        use std::os::raw::c_char;
        unsafe { CStr::from_ptr(concat!($lit, "\0").as_ptr() as *const c_char).as_ptr() }
    }};
}

#[macro_export]
macro_rules! check_session {
    ($session:expr) => {
        use crate::ffi::{HAPI_IsSessionValid, HAPI_Result};
        assert!(
            unsafe {
                matches!(
                    HAPI_IsSessionValid($session),
                    HAPI_Result::HAPI_RESULT_SUCCESS
                )
            },
            "Session is invalid!"
        );
    };
}
