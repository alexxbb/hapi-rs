#![doc(html_logo_url = "https://media.sidefx.com/uploads/products/engine/engine_orange.svg")]
//! # Rust bindings to Houdini Engine C API.
//!
//! Official HAPI [documentation](https://www.sidefx.com/docs/hengine/):
//!
//! Check out the [examples](https://github.com/alexxbb/hapi-rs/tree/main/examples):
//!
//! `cargo run --examples ...`
//!
//!
//! # Building and running
//!
//! **HFS** environment variable must be set for the build script to link to Houdini libraries.
//!
//! For runtime discovery of Houdini libraries there are several options:
//!
//! **Option 1**
//!
//! *Build with RPATH via the RUSTFLAGS variable:*
//! ```bash
//! RUSTFLAGS="-C link-args=-Wl,-rpath,/path/to/hfs/dsolib" cargo build
//! ```
//! **Option 2**
//!
//! *Add a cargo config file to your project: `.cargo/config`*
//!```ignore
//! [target.'cfg(target_os = "linux")']
//! rustflags = ["-C", "link-arg=-Wl,-rpath=/opt/hfs/19.0.455/dsolib"]
//! [target.x86_64-apple-darwin]
//! rustflags = ["-C",
//!     "link-arg=-Wl,-rpath,/Applications/Houdini/Current/Frameworks/Houdini.framework/Versions/Current/Libraries",
//! ]
//! [target.x86_64-pc-windows-msvc]
//! rustflags = ["-C", "link-arg=-Wl,-rpath,C:/Houdini/19.0.455/custom/houdini/dsolib"]
//!```
//! **Option 3**
//!
//! At runtime via env variables: `$PATH` on windows, `$LD_LIBRARY_PATH` on Linux and `$DYLD_LIBRARY_PATH` on MacOS
//!
//! # API Overview
//!
//! ```Note: Currently PDG APIs are not yet implemented.```
//!
//! This crates tries hard to be nice and easy to use, hiding the inconvenient C API as much as possible
//! while also trying to keep function names clear and close to original.
//! To archive this, the crate wraps every single C struct in a new struct and provide getters/setters for its fields.
//! All structs and enums have their `HAPI_` prefix removed.
//!
//! In addition all enum variants are shortened.
//! For example:
//! ```ignore
//! // Original struct:
//! struct HAPI_NodeInfo {
//!    pub parmCount: ::std::os::raw::c_int,
//!    ...
//! }
//! // This crate's struct:
//! let info: crate::node::NodeInfo;
//! let count: i32 = info.parm_count();
//!
//! // Original enum (C)
//!  enum HAPI_InputType {
//!       HAPI_INPUT_INVALID = -1,
//!       HAPI_INPUT_TRANSFORM,
//!       HAPI_INPUT_GEOMETRY,
//!       HAPI_INPUT_MAX
//! };
//! // This crate's enum
//! enum InputType {
//!     Invalid = -1,
//!     Transform = 0,
//!     Geometry = 1,
//!     Max = 2
//! }
//! ```
//! Also some structs, don't provide a direct way of creating them as well as missing setters because while it's possible to create them in C (and in Rust)
//! it doesn't make sense from a usability point of view, e.g you never need to create and modify a [`node::NodeInfo`] struct.
//! Structs that you do need ability to create, implement [Default] and follow the `Builder Pattern` with convenient `with_` and `set_` methods:
//! ```ignore
//! let part_info = PartInfo::default()
//!    .with_part_type(PartType::Mesh)
//!    .with_face_count(6);
//! ```
//!
//! # Error type
//! All API calls return [`HapiError`] ([HAPI_Result](https://www.sidefx.com/docs/hengine/_h_a_p_i___common_8h.html#ac52e921ba2c7fc21a0f245678f76c836))
//! In case of error, the HapiError struct contains an Option<String> with an error message returned from the Engine.
//!
//!
//! # Strings
//! Houdini Engine being C API, makes life a little harder for Rust programmer when it comes to strings.
//! The crate chose to accept some overhead related to string conversion in exchange for a nicer API and
//! easy of use.
//!
//! For example getting/setting a parameter value will perform a conversion CString <-> String,
//! but not in every situation such conversion is acceptable, for example reading geometry string attributes
//! can be very expensive since we have do potentially thousands of CString to String conversions.
//!
//! To aid this situation, the crate provides custom structs which implement different iterators,
//! which return CString or String. See the [`stringhandle`] module for more info.
//!
//!
extern crate core;

pub mod asset;
pub mod attribute;
pub mod geometry;
pub mod material;
pub mod node;
pub mod parameter;
pub mod session;
pub mod stringhandle;
pub mod volume;
pub mod pdg;
mod errors;
mod ffi;

pub use errors::{HapiError, Result, ErrorContext};
pub use ffi::enums;

/// Houdini version this library was build upon
#[derive(Debug)]
pub struct HoudiniVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    pub patch: u32,
}

/// Engine version this library was build upon
#[derive(Debug)]
pub struct EngineVersion {
    pub major: u32,
    pub minor: u32,
    pub api: u32,
}

/// Houdini version this library was build upon
pub const HOUDINI_VERSION: HoudiniVersion = HoudiniVersion {
    major: ffi::raw::HAPI_VERSION_HOUDINI_MAJOR,
    minor: ffi::raw::HAPI_VERSION_HOUDINI_MINOR,
    build: ffi::raw::HAPI_VERSION_HOUDINI_BUILD,
    patch: ffi::raw::HAPI_VERSION_HOUDINI_PATCH,
};

/// Engine version this library was build upon
pub const ENGINE_VERSION: EngineVersion = EngineVersion {
    major: ffi::raw::HAPI_VERSION_HOUDINI_ENGINE_MAJOR,
    minor: ffi::raw::HAPI_VERSION_HOUDINI_ENGINE_MINOR,
    api: ffi::raw::HAPI_VERSION_HOUDINI_ENGINE_API,
};
