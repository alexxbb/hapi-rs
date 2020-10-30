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
        use std::ffi::CStr;
        use std::os::raw::c_char;
        unsafe { CStr::from_ptr(concat!($lit, "\0").as_ptr() as *const c_char).as_ptr() }
    }};
}

mod asset;
mod cookoptions;
mod errors;
mod extentions;
mod node;
mod session;
mod status;
mod stringhandle;

pub use cookoptions::CookOptions;
pub use errors::{HAPI_Error, Kind, Result};
pub(crate) use extentions::*;
pub use session::{Initializer, Session};
pub use stringhandle::get_string;

#[cfg(test)]
mod tests {
    #[test]
    fn foo() {
        assert_eq!(1, 1)
    }
}
