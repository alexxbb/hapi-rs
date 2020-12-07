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
        use crate::ffi::{HAPI_IsSessionValid, HapiResult};
        assert!(
            unsafe { matches!(HAPI_IsSessionValid($session), HapiResult::Success) },
            "Session is invalid!"
        );
    };
}

#[macro_export]
macro_rules! builder {
    (
        @name: $builder:ident, @ffi: $inner:ty, @default: $default:block, @result: $object:ident,
        @setters: {
            $($method:ident->$ffi_field:ident: $ret:ty),+ $(,)? // optional comma
        }
    ) => {
        pub struct $builder($inner);
        impl Default for $builder {
            fn default() -> Self {
                Self(unsafe { $default })
            }
        }
        impl $builder {
            $(pub fn $method(mut self, val: $ret) -> Self {
                self.0.$ffi_field = val;
                self
            })+
        }

        impl $builder {
            pub fn build(mut self) -> $object {
                $object{inner: self.0}
            }
        }
    };
}

#[macro_export]
macro_rules! wrap_ffi {
    ($object:ident, $self_:ident, $($method:ident->$ret:ty $block:block),+ $(,)?) => {
        impl $object {
            $(
                pub fn $method(&$self_) -> $ret {
                    $block
                }
            )+
        }
    };
}
