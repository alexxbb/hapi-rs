#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod cookoptions;
// mod session;
// mod errors;

mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use cookoptions::CookOptionsBuilder;

#[cfg(test)]
mod tests {}
