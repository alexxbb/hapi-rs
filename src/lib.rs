#![doc(html_logo_url = "https://media.sidefx.com/uploads/products/engine/engine_orange.svg")]
//! # Rust bindings to Houdini Engine C API.
//!
//! Official HAPI [documentation](https://www.sidefx.com/docs/hengine/):
//!
//! Check out the [examples](https://github.com/alexxbb/hapi-rs/tree/main/examples):
//!
//! `cargo run --example ...`
//!
//!
//! # Building and running
//!
//! **HFS** environment variable must be set for the build script to link to Houdini libraries.
//!
//! For runtime discovery of Houdini libraries there are several options:
//!
//! ## Mac & Linux
//!
//! **Option 1**
//!
//! Build with RPATH via the RUSTFLAGS variable:
//! ```bash
//! RUSTFLAGS="-C link-args=-Wl,-rpath,/path/to/hfs/dsolib" cargo build
//! ```
//! **Option 2**
//!
//! Add a cargo config file to your project: `.cargo/config`
//!```text
//! [target.'cfg(target_os = "linux")']
//! rustflags = ["-C", "link-arg=-Wl,-rpath=/opt/hfs/20.5.445/dsolib"]
//! [target.x86_64-apple-darwin]
//! rustflags = ["-C",
//!     "link-arg=-Wl,-rpath,/Applications/Houdini/Current/Frameworks/Houdini.framework/Versions/Current/Libraries",
//! ]
//!```
//! **Option 3**
//!
//! At runtime via env variables: `$LD_LIBRARY_PATH` on Linux and `$DYLD_LIBRARY_PATH` on MacOS
//!
//! ## Windows
//! `$HFS` variable must be set for building.
//! Runtime Houdini libraries are required to be in $PATH: Add $HFS/bin to $PATH.
//!
//! # API Overview
//!
//! This crates aims to be easy to use, hiding the inconvenient C API as much as possible
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
//!       ...
//! };
//! // This crate's enum
//! enum InputType {
//!     Invalid = -1,
//!     ...
//! }
//! ```
//! Some underlying C structs don't provide a direct way of creating them or they might not provide methods
//! for modifying them, due to this crate's attempt for proper data encapsulation, minimizing noise and improving safety.
//! Structs that you **do** need the ability to create, implement [Default] and some a `Builder Pattern` with convenient `with_` and `set_` methods:
//! ```ignore
//! let part_info = PartInfo::default()
//!    .with_part_type(PartType::Mesh)
//!    .with_face_count(6);
//! ```
//!
//! # Error type
//! All API calls return [`HapiError`] ([HAPI_Result](https://www.sidefx.com/docs/hengine/_h_a_p_i___common_8h.html#ac52e921ba2c7fc21a0f245678f76c836))
//! In case of error, the HapiError struct contains an `Option<String>` with an error message returned from the Engine.
//! Additional error context is available in the `contexts` field of the error type.
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
//! which yield CString or String types. See the [`stringhandle`] module for more info.
//!
//!
//! # Nodes
//! [`node::NodeHandle`] is lightweight handle to a Houdini node, it's returned by some APIs and can not
//! be created by the users. The [`node::NodeHandle`] has a limited functionality because it doesn't hold
//! a pointer to the [`session::Session`].
//!
//! [`node::HoudiniNode`] is the main type which combines [`node::NodeHandle`], [`node::NodeInfo`]
//! and [`session::Session`] types into a single struct which exports most of the node APIs.
//!
//! Some APIs choose to return a lightweight [`node::NodeHandle`] that can be simply passed to other APIs.
//! You can upgrade the handle to a full node by [`node::NodeHandle::to_node`] to get access to full node API.
//!
//! Due to HAPI only exposes a limited subset of APIs to Houdini, and to keep things simple,
//! there no different flavors of [`node::HoudiniNode`].
//! Instead this type provides the common node APIs and some node-type-specific APIs are available
//! in separate types like [`pdg::TopNode`] and [`geometry::Geometry`] which hold [`node::HoudiniNode`]
//! as a struct field (composition vs inheritance).
//!
//! # Parameters
//! Parameters are modelled with [`parameter::Parameter`] enum that contains different parameter types.
//! Common parameter methods are provided in [`parameter::ParmBaseTrait`].

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
mod utils;
mod ffi;

pub use errors::{HapiError, Result};
pub use ffi::enums;
pub use ffi::raw;
pub use ffi::structs::Viewport;

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
    major: raw::HAPI_VERSION_HOUDINI_MAJOR,
    minor: raw::HAPI_VERSION_HOUDINI_MINOR,
    build: raw::HAPI_VERSION_HOUDINI_BUILD,
    patch: raw::HAPI_VERSION_HOUDINI_PATCH,
};

/// Engine version this library was build upon
pub const ENGINE_VERSION: EngineVersion = EngineVersion {
    major: raw::HAPI_VERSION_HOUDINI_ENGINE_MAJOR,
    minor: raw::HAPI_VERSION_HOUDINI_ENGINE_MINOR,
    api: raw::HAPI_VERSION_HOUDINI_ENGINE_API,
};
