#![doc(html_logo_url = "https://media.sidefx.com/uploads/products/engine/engine_orange.svg")]
//! SideFX Houdini Meets Rust!
//!
//! [SideFx Houdini](https://www.sidefx.com/) is a world leading software for creating stunning visual effects for movies and games.
//! Apart from the main graphical interface written in C++ and Python, Houdini also provides a C interface called [Houdini Engine](https://www.sidefx.com/products/houdini-engine/) or HAPI for short.
//! Its goal is to bring the power of Houdini to other DCCs (Digital Content Creation) software and game engines.
//!
//! This crate aims to provide idiomatic Rust interface to HAPI and is built on top of [hapi-sys](https://crates.io/crates/hapi-sys),
//! but **it doesn't depend on it**, i.e. the generated bindings file from `hapi-sys` is included in this crate.
//!
//! **⚠ A valid **commercial** Houdini Engine license is required to use this crate ⚠**
//!
//! Thanks to Rust's powerful type system using the engine from Rust is very simple
//!
//! # Example
//! ```ignore
//! use hapi_rs::Result;
//! use hapi_rs::session::quick_session;
//! use hapi_rs::parameter::*;
//!
//! fn main() -> Result<()> {
//!     // Quick session starts a standalone engine process
//!     let session = quick_session(None)?;
//!     // Load a Houdini Asset, create a node
//!     session.load_asset_file("otls/hapi_geo.hda")?;
//!     let node = session.create_node_blocking("Object/hapi_geo", None, None)?;
//!     // Set the "scale" parameter
//!     if let Parameter::Float(parm) = node.parameter("scale")? {
//!         parm.set_value(&[3.0])?;
//!         node.cook(None)?;
//!     }
//!     // Get a reference to the node's internal geometry
//!     let geometry = node.geometry()?.expect("geometry");
//!     // Save it as one of the supported geometry formats
//!     geometry.save_to_file("/tmp/output.fbx")?;
//!     Ok(())
//! }
//! ```
//!
//! # Building
//!
//! **HFS** environment variable must be set for build script to link to Houdini libraries.
//! Also runtime libraries are searched in `$PATH` on windows, `$LD_LIBRARY_PATH` on Linux and `$DYLD_LIBRARY_PATH` on MacOS
//!
//! # Design Overview
//! This crates tries hard to be nice and easy to use, hiding the inconvenient C API as much as possible
//! while also trying to keep function names clear and close to original.
//! To archive this, the crate wraps every single bindgen-generated C struct in a new struct and provide getters/setters for its fields.
//! All structs and enums have their `HAPI_` prefix removed.
//!
//! In addition all enum variants are shortened. This is done by custom post-processing in [hapi-sys](https://crates.io/crates/hapi-sys)
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
//! it doesn't make sense from a usability point of view, i.e you never need to create and modify a [`node::NodeInfo`] struct.
//! Structs that you do need ability to create, implement [Default] and have `with_` and `set_` methods:
//! ```ignore
//! let part_info = PartInfo::default()
//!    .with_part_type(PartType::Mesh)
//!    .with_face_count(6);
//! ```
//!
//! # Session
//! Engine [promises](https://www.sidefx.com/docs/hengine/_h_a_p_i__sessions.html#HAPI_Sessions_Multithreading)
//! to be thread-safe when accessing a single `Session` from multiple threads.
//! `hapi-rs` relies on this promise and the [session::Session] struct holds only an `Arc` pointer to the session,
//! so it's `Send` and `Sync` in Rust, and a few occasions it internally uses a [parking_lot::ReentrantMutex] for
//! making sure a series of API calls from the same thread are sequential.
//!
//! When the last instance of the `Session` is about to drop, it'll be cleaned
//! (if [session::SessionOptions::cleanup] was set) and automatically closed.
//!
//! The Engine process (pipe or socket) can be auto-terminated as well if told so when starting the server:
//! See [session:start_engine_pipe_server] and [session::start_engine_socket_server]
//!
//! [session::quick_session] terminates the server by default. This is useful for quick one-off jobs.
//!
//!
//! # Error type
//! All API calls return [`HapiError`] ([HAPI_Result](https://www.sidefx.com/docs/hengine/_h_a_p_i___common_8h.html#ac52e921ba2c7fc21a0f245678f76c836))
//! Moreover, in case of error, the HapiError struct keeps a pointer to [session::Session] to retrieves the error message from the engine ad hoc.
//!
mod errors;
mod ffi;
mod stringhandle;
pub mod asset;
pub mod attribute;
pub mod geometry;
pub mod node;
pub mod parameter;
pub mod session;
pub mod material;

pub use errors::{Result, HapiError, Kind};
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
