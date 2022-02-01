#![doc(html_logo_url = "https://media.sidefx.com/uploads/products/engine/engine_orange.svg")]
//! SideFX Houdini Meets Rust!
//!
//! [SideFx Houdini](https://www.sidefx.com/) is a world leading software for creating stunning visual effects for movies and games.
//! Apart from the main graphical interface written in C++ and Python, Houdini also provides a C interface called [Houdini Engine](https://www.sidefx.com/products/houdini-engine/) or HAPI for short.
//! Its goal is to bring the power of Houdini to other DCCs (Digital Content Creation) software and game engines.
//!
//! This crate aims to provide idiomatic Rust interface to Houdini Engine and is built on top of [hapi-sys](https://crates.io/crates/hapi-sys).
//!
//! **⚠ A valid **commercial** Houdini Engine license is required to use this crate ⚠**
//!
//! Thanks to Rust's powerful type system using the engine from Rust is very straightforward.
//!
//! # Example
//! ```ignore
//! use hapi_rs::Result;
//! use hapi_rs::session::quick_session;
//! use hapi_rs::parameter::*;
//!
//! fn main() -> Result<()> {
//!     // Quick session starts a standalone engine process
//!     let session = quick_session()?;
//!     // Load a Houdini Asset, create a node
//!     session.load_asset_file("otls/hapi_geo.hda")?;
//!     let node = session.create_node("Object/hapi_geo", None, None)?;
//!     // Set the "scale" parameter
//!     if let Parameter::Float(parm) = node.parameter("scale")? {
//!         parm.set_value(&[3.0])?;
//!         node.cook(None)?;
//!     }
//!     // Get a reference to the node's internal geometry
//!     let geometry = node.geometry()?.expect("geometry");
//!     // Save it to one of the supported geometry formats
//!     geometry.save_to_file("/tmp/output.fbx")?;
//!     Ok(())
//! }
//! ```
//! Check out the other examples:
//!
//! `cargo run --examples ...`
//!
//!
//! # Building and running
//!
//! **HFS** environment variable must be set for build script to link to Houdini libraries.
//!
//! For runtime Houdini libraries there are several options:
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
//! # API Coverage
//! Currently PDG APIs are not yet implemented.
//!
//! # Design Overview
//! This crates tries hard to be nice and easy to use, hiding the inconvenient C API as much as possible
//! while also trying to keep function names clear and close to original.
//! To archive this, the crate wraps every single C struct in a new struct and provide getters/setters for its fields.
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
//! Structs that you do need ability to create, implement [Default] and follow the `Builder Pattern` with convenient `with_` and `set_` methods:
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
//! and *does not* protect the session with Mutex, although there is a [parking_lot::ReentrantMutex]
//! private member which is used internally in a few cases where API calls must be sequential.
//!
//! When the last instance of the `Session` is about to drop, it'll be cleaned
//! (if [session::SessionOptions::cleanup] was set) and automatically closed.
//!
//! The Engine process (pipe or socket) can be auto-terminated as well if told so when starting the server:
//! See [session:start_engine_pipe_server] and [session::start_engine_socket_server]
//!
//! [session::quick_session] terminates the server by default. This is useful for quick one-off jobs.
//!
//! # Nodes
//! Houdini nodes are represented as [`node::HoudiniNode`] struct and all node-related functions are
//! methods on that struct. It has a public `info` field with [`node::NodeInfo`] with details about the node.
//!
//! See the [node] module for details.
//!
//! # Geometry
//! [`geometry::Geometry`] is a wrapper around `HoudiniNode` with methods for accessing geometry.
//!
//! See the [geometry] module for details.
//!
//!
//! # Error type
//! All API calls return [`HapiError`] ([HAPI_Result](https://www.sidefx.com/docs/hengine/_h_a_p_i___common_8h.html#ac52e921ba2c7fc21a0f245678f76c836))
//! Moreover, **only** in case of error, the HapiError struct will keep a pointer to [session::Session] to retrieves the error message from the engine ad hoc.
//!
//!
//! # Strings
//! Houdini Engine being C API, makes life harder for Rust when it comes to strings.
//! The crate chose to accept some overhead related to string conversion in exchange for a nicer API and
//! easy of use.
//!
//! For example getting/setting a parameter value will perform a conversion CString <-> String,
//! but not in every situation such conversion is acceptable, for example reading heavy geometry string attributes
//! can be very expensive since we have do potentially thousands of CString to String conversions.
//!
//! To aid this situation, the crate provides custom structs which implement different iterators,
//! which return CString or String. See the [`stringhandle`] module for more info.
//!
//!
pub mod asset;
pub mod attribute;
mod errors;
mod ffi;
pub mod geometry;
pub mod material;
pub mod node;
pub mod parameter;
pub mod session;
pub mod stringhandle;
pub mod volume;

pub use errors::{HapiError, Kind, Result};
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
