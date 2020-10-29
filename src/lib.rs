#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod cookoptions;
mod session;
mod errors;
mod status;

mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use session::{Session, Initializer};
pub use cookoptions::CookOptions;
pub use errors::{HAPI_Error, Result};

#[cfg(test)]
mod tests {}
