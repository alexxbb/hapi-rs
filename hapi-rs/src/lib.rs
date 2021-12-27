#![doc(html_logo_url = "https://media.sidefx.com/uploads/products/engine/engine_orange.svg")]
//! SideFX Houdini Meets Rust!
//!
//! [SideFx Houdini](https://www.sidefx.com/) is a world leading software for creating stunning visual effects for movies and games.
//! Apart from the main graphical interface written in C++ and Python, Houdini also provides a C interface called [Houdini Engine](https://www.sidefx.com/products/houdini-engine/) or HAPI for short.
//! Its goal is to provide the power of Houdini to other DCCs (Digital Content Creation) software and game engines.
//!
//! This crate aims to provide idiomatic Rust interface to HAPI and is built on top of [hapi-sys](https://crates.io/crates/hapi-sys)
//!
//! **⚠ A valid **commercial** Houdini Engine license is required to use this crate ⚠**
//!
//! Thanks to Rust's powerful type system using the engine from Rust is very simple
//!
//! # Example
//! ```
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
//! # Design Overview
//! This crates tries hard to be nice and easy to use, hiding the ugly C API as much as possible
//! while also trying to keep function names clear and close to original.
//! Also I'm very allergic to camelCase and this crate has zero of them
//! To archive this, the crate wraps every single bindgen-generated C struct in a new struct and provide getters/setters for its fields.
//! For example:
//! ```
//! struct HAPI_NodeInfo {
//!    pub parmCount: ::std::os::raw::c_int,
//!    ...
//! }
//! ```
//! Becomes:
//! ```
//! let info: NodeInfo;
//! let count: i32 = info.parm_count();
//! ```
//! Also many structs, don't provide setters because while it's possible to create them in C (and in Rust)
//! it doesn't make sense from a usability point of view, i.e you never need to modify a [`node::NodeInfo`] struct.
//! Structs that you do need ability to create, implement [Default] and have `with_` methods:
//! ```
//! let part_info = PartInfo::default()
//!    .with_part_type(PartType::Mesh)
//!    .with_face_count(6);
//! ```
//!
//! # Thread Safety
//! Engine C API promises to be thread-safe when accessing a single `Session` from multiple threads.
//! Rust relies on this promise and the [Session] struct holds only an `Arc` pointer inside,
//! so it's `Send` and `Sync` in Rust, however in very few cases it uses a parking_lot::ReentrantMutex for
//! making sure a series of API calls from a thread are sequential
//! See [`session::Session`] for more information.
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
