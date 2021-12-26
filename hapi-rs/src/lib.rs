mod errors;
mod ffi;
mod stringhandle;
pub mod asset;
pub mod attribute;
pub mod geometry;
pub mod node;
pub mod parameter;
pub mod session;

pub use errors::Result;
pub use ffi::enums;

#[derive(Debug)]
pub struct HoudiniVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    pub patch: u32,
}

#[derive(Debug)]
pub struct EngineVersion {
    pub major: u32,
    pub minor: u32,
    pub api: u32,
}

pub const HOUDINI_VERSION: HoudiniVersion = HoudiniVersion {
    major: ffi::raw::HAPI_VERSION_HOUDINI_MAJOR,
    minor: ffi::raw::HAPI_VERSION_HOUDINI_MINOR,
    build: ffi::raw::HAPI_VERSION_HOUDINI_BUILD,
    patch: ffi::raw::HAPI_VERSION_HOUDINI_PATCH,
};

pub const ENGINE_VERSION: EngineVersion = EngineVersion {
    major: ffi::raw::HAPI_VERSION_HOUDINI_ENGINE_MAJOR,
    minor: ffi::raw::HAPI_VERSION_HOUDINI_ENGINE_MINOR,
    api: ffi::raw::HAPI_VERSION_HOUDINI_ENGINE_API,
};
