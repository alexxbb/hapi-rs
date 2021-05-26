// #![allow(unused)]
pub mod asset;
pub mod attribute;
mod errors;
pub mod ffi;
pub mod geometry;
pub mod node;
pub mod parameter;
pub mod session;
mod stringhandle;
#[cfg(test)]
mod tests;

pub use errors::Result;
pub use stringhandle::get_string;

pub use crate::ffi::{
    TimelineOptions
};

pub use crate::ffi::raw::{
    HapiResult, NodeFlags, NodeType, State, StatusType, StatusVerbosity, StorageType, PartType
};

#[derive(Debug)]
pub struct _HoudiniVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    pub patch: u32,
}

#[derive(Debug)]
pub struct _EngineVersion {
    pub major: u32,
    pub minor: u32,
    pub api: u32,
}

pub const HOUDINI_VERSION: _HoudiniVersion = _HoudiniVersion {
    major: ffi::raw::HAPI_VERSION_HOUDINI_MAJOR,
    minor: ffi::raw::HAPI_VERSION_HOUDINI_MINOR,
    build: ffi::raw::HAPI_VERSION_HOUDINI_BUILD,
    patch: ffi::raw::HAPI_VERSION_HOUDINI_PATCH,
};

pub const ENGINE_VERSION: _EngineVersion = _EngineVersion {
    major: ffi::raw::HAPI_VERSION_HOUDINI_ENGINE_MAJOR,
    minor: ffi::raw::HAPI_VERSION_HOUDINI_ENGINE_MINOR,
    api: ffi::raw::HAPI_VERSION_HOUDINI_ENGINE_API,
};

#[allow(unused)]
macro_rules! char_ptr {
    ($lit:expr) => {{
        use std::ffi::CStr;
        use std::os::raw::c_char;
        unsafe { CStr::from_ptr(concat!($lit, "\0").as_ptr() as *const c_char).as_ptr() }
    }};
}
