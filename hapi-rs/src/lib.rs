#![allow(unused)]
use log;
#[macro_use]
mod macros;
#[macro_use]
pub mod errors;
mod asset;
pub mod node;
pub mod session;
mod stringhandle;
mod attribute;
pub mod parameter;
#[cfg(test)]
mod tests;
pub mod ffi;

pub use errors::Result;
pub use stringhandle::get_string;

pub use crate::ffi::raw::{
    NodeFlags, NodeType,
    StorageType, State, StatusType, StatusVerbosity, HapiResult
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
