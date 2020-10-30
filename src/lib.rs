#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#![allow(dead_code)]
#![allow(clippy::all)]
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[macro_export]
macro_rules! char_ptr {
    ($lit:expr) => {{
        use std::os::raw::c_char;
        use std::ffi::CStr;
        unsafe { CStr::from_ptr(concat!($lit, "\0").as_ptr() as *const c_char).as_ptr() }
    }}
}

mod cookoptions;
mod session;
mod errors;
mod status;
mod extentions;
mod asset;
mod stringhandle;


pub use session::{Session, Initializer};
pub use cookoptions::CookOptions;
pub use errors::{HAPI_Error, Result};
pub use stringhandle::get_string;
pub(crate) use extentions::*;

#[cfg(test)]
mod tests {
    #[test]
    fn foo() {
        assert_eq!(1, 1)
    }
}
