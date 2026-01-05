#![doc(html_logo_url = "https://media.sidefx.com/uploads/products/engine/engine_orange.svg")]
//! # Rust bindings to Houdini Engine C API.
//!
//! Official HAPI [documentation](https://www.sidefx.com/docs/hengine/) is bundled under
//! `engine_docs/www.sidefx.com/docs/hengine/` for offline reference.
//!
//! Check out the [examples](https://github.com/alexxbb/hapi-rs/tree/main/lib/examples):
//!
//! `cargo run --example ...`
//!
//! ## Building and running
//!
//! **HFS** environment variable must be set for the build script to link to Houdini libraries.
//!
//! For runtime discovery of Houdini libraries there are several options:
//!
//! ### Mac & Linux
//!
//! **Option 1**
//!
//! Build with RPATH via the `RUSTFLAGS` variable:
//! ```bash
//! RUSTFLAGS="-C link-args=-Wl,-rpath,/path/to/hfs/dsolib" cargo build
//! ```
//! **Option 2**
//!
//! Add a cargo config file to your project: `.cargo/config`
//!```text
//! [target.'cfg(target_os = "linux")']
//! rustflags = ["-C", "link-arg=-Wl,-rpath=/opt/hfs/21.0.440/dsolib"]
//! [target.x86_64-apple-darwin]
//! rustflags = ["-C",
//!     "link-arg=-Wl,-rpath,/Applications/Houdini/Current/Frameworks/Houdini.framework/Versions/Current/Libraries",
//! ]
//!```
//! **Option 3**
//!
//! At runtime via env variables: `$LD_LIBRARY_PATH` on Linux and `$DYLD_LIBRARY_PATH` on macOS.
//!
//! ### Windows
//! `$HFS` must be set for building.
//! Runtime Houdini libraries have to be in `$PATH`, e.g. add `$HFS/bin`.
//!
//! ## Architectural overview
//! `hapi-rs` mirrors the Houdini Engine C API (HAPI) while providing more idiomatic Rust ergonomics:
//! - Every `HAPI_*` struct becomes a dedicated Rust wrapper with `get_*`, `set_*`, and `with_*` helpers
//!   (for example `HAPI_NodeInfo` -> [`node::NodeInfo`], `HAPI_GeoInfo` -> [`geometry::GeoInfo`]).
//! - Enums drop the `HAPI_` prefix and use concise variant names while preserving the C ordering so that
//!   official documentation can be cross-referenced easily.
//! - Most structs implement [`Default`] and expose builder-like methods to keep initialization readable.
//! - String conversions are centralized in [`stringhandle`] to hide `HAPI_StringHandle` management and to
//!   provide iterators that either keep data as `CString` or eagerly convert to `String` when needed.
//! - [`asset`], [`session`], [`node`], [`geometry`], [`attribute`], [`parameter`], [`material`], [`volume`],
//!   and [`pdg`] modules roughly follow the sections from the Houdini Engine manual so that the mental model
//!   stays close to the C API.
//! - Raw HAPI symbols remain accessible through [`raw`], the bindgen-generated module re-exported for
//!   advanced integrations when you need to drop straight to the C API.
//!
//! ## Sessions and servers
//! The [`session::Session`] type encapsulates a live connection to Houdini Engine. Internally it stores an
//! `Arc<SessionInner>` so cloning a session is cheap and the connection stays alive until the last clone is
//! dropped. Houdini guarantees that a single session handle can be used concurrently; consequently the Rust
//! wrapper is [`Send`] + [`Sync`] and only falls back to a private [`parking_lot::ReentrantMutex`] when HAPI
//! needs serialized calls. When [`session::SessionOptions::cleanup`] is enabled (the default), dropping the
//! last clone will automatically clean up the session and shut down the associated server.
//!
//! [`session::simple_session`] bootstraps a Thrift shared-memory server via [`session::new_thrift_session`]
//! and [`server::ServerOptions::shared_memory_with_defaults`]. You can override transports, buffer sizes,
//! environment variables, or timeouts with [`server::ServerOptions`]—see `lib/examples/setup_server.rs` for
//! an advanced configuration.
//!
//! License preference can be set with [`server::ServerOptions::with_license_preference`].
//!
//! Sessions expose helpers that map closely to the HAPI entry points:
//! - [`session::Session::load_asset_file`] returns an [`asset::AssetLibrary`] so you can instantiate HDAs.
//! - [`session::Session::create_node`], [`session::Session::node_builder`], and
//!   [`session::Session::create_input_node`] for editable geometry inputs.
//! - [`session::Session::set_server_var`] / [`session::Session::get_server_var`] variable APIs.
//! - [`session::Session::cook`] reports [`session::CookResult`] when you run in threaded mode.
//!
//! ```rust
//! use hapi_rs::session::simple_session;
//! use std::path::PathBuf;
//!
//! fn main() -> hapi_rs::Result<()> {
//!     let session = simple_session().unwrap();
//!     assert!(session.is_valid());
//!     let hda = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../otls/hapi_geo.hda");
//!     let library = session.load_asset_file(&hda)?;
//!     let node = library.try_create_first()?;
//!     node.cook_blocking()?;
//!     Ok(())
//! }
//! ```
//!
//! ## HoudiniNode and node APIs
//! HAPI node ids become [`node::NodeHandle`], a lightweight wrapper that you often receive when traversing
//! networks. Call [`node::NodeHandle::to_node`] to promote the handle into a [`node::HoudiniNode`], which
//! stores the handle, [`node::NodeInfo`], and the owning [`session::Session`]. `HoudiniNode` is also
//! [`Clone`] + [`Send`] + [`Sync`] so you can keep references across threads or store them inside higher
//! level abstractions such as [`geometry::Geometry`] or [`pdg::TopNode`].
//!
//! Nodes expose the most commonly used HAPI functionality:
//! - [`node::HoudiniNode::cook_blocking`] / [`node::HoudiniNode::cook_with_options`] forward to
//!   `HAPI_CookNode` and integrate with [`session::Session::cook`] for threaded servers.
//! - [`node::HoudiniNode::geometry`] returns a [`geometry::Geometry`] for SOP nodes or follows the display flag
//!   when called on OBJ nodes.
//! - [`node::HoudiniNode::parameters`] retrieves strongly-typed [`parameter::Parameter`] values.
//! - Networking helpers such as [`node::HoudiniNode::find_children_by_type`] or [`node::ManagerNode`] mirror
//!   the C API utilities.
//! - File IO helpers ([`node::HoudiniNode::save_to_file`], [`node::HoudiniNode::load_from_file`]) keep the
//!   naming/parsing identical to HAPI.
//!
//! For complete node-network demos see `lib/examples/node_networks.rs` (wiring SOPs), `lib/examples/node_errors.rs`
//! (reading cook errors), and `lib/examples/live_session.rs` (re-using an existing Houdini session).
//!
//! ## Geometry workflow
//! [`geometry::Geometry`] represents SOP outputs and wraps [`geometry::GeoInfo`] so you get immediate access to
//! element counts, part metadata, group names, or material assignments. When you create a node from
//! `../otls/hapi_geo.hda`, [`node::HoudiniNode::geometry`] either returns the SOP node directly or finds the
//! display child for OBJ nodes. [`session::Session::create_input_node`] and
//! [`session::Session::create_input_curve_node`] expose editable SOP nodes if you want to push points back
//! into Houdini.
//!
//! Each geometry is composed of [`geometry::PartInfo`] entries (points, meshes, curves, packed primitives, …).
//! Use [`geometry::Geometry::partitions`] or [`geometry::Geometry::part_info`] to explore them, then rely on
//! helpers such as [`geometry::Geometry::get_materials`] to inspect [`geometry::Materials`]. Materials can be
//! promoted to [`material::Material`] for texture extraction—`lib/examples/materials.rs` shows how to render
//! maps from `../otls/sesi/SideFX_spaceship.hda`.
//!
//! The curve and group APIs map directly to HAPI (`get_curve_counts`, `get_group_names`, etc.); see
//! `lib/examples/object_geos_parts.rs` and `lib/examples/curve_output.rs` for in-depth geometry traversals.
//!
//! ```rust
//! use hapi_rs::{
//!     geometry::PartType,
//!     session::simple_session,
//! };
//! use std::path::PathBuf;
//!
//! fn main() -> hapi_rs::Result<()> {
//!     let session = simple_session().unwrap();
//!     let hda = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../otls/hapi_geo.hda");
//!     let library = session.load_asset_file(&hda)?;
//!     let node = library.try_create_first()?;
//!     node.cook_blocking()?;
//!     let geometry = node.geometry()?.expect("SOP geometry");
//!     let mesh = geometry
//!         .partitions()?
//!         .into_iter()
//!         .find(|part| part.part_type() == PartType::Mesh)
//!         .expect("mesh part");
//!     assert!(mesh.point_count() > 0);
//!     Ok(())
//! }
//! ```
//! Some underlying C structs don't provide a direct way of creating them or they might not provide methods
//! for modifying them, due to this crate's attempt for proper data encapsulation, minimizing noise and improving safety.
//! Structs that you **do** need the ability to create, implement [Default] and some implement a `Builder Pattern` with convenient `with_` and `set_` methods:
//! ```rust
//! use hapi_rs::geometry::{PartInfo, PartType};
//! let part_info = PartInfo::default()
//!    .with_part_type(PartType::Mesh)
//!    .with_face_count(6);
//! ```
//!
//! ## Attribute access and type-safe downcasting
//! Attributes come back as a dynamic [`attribute::Attribute`] wrapper. You can inspect its storage using
//! [`attribute::Attribute::storage`] and downcast it into concrete types such as [`attribute::NumericAttr`],
//! [`attribute::StringAttr`], or [`attribute::DictionaryAttr`]. This mirrors the HAPI concept of inspecting
//! [`geometry::AttributeInfo`] first and then choosing the right getter; the downcast enforces the check at
//! compile time.
//!
//! [`geometry::Geometry::get_attribute`] will fetch any attribute by owner/name, while convenience helpers such
//! as [`geometry::Geometry::get_position_attribute`] cover common cases. To create new attributes you build an
//! [`geometry::AttributeInfo`] (it implements [`Default`] + builder setters), then call one of the
//! `add_*_attribute` methods. Array and dictionary attributes lean on [`attribute::DataArray`] and the
//! async helpers in [`attribute::async_]` for large transfers. Examples like `lib/examples/curve_output.rs` and
//! `lib/examples/groups.rs` showcase real-world usages.
//!
//! ```rust
//! use hapi_rs::{
//!     attribute::NumericAttr,
//!     geometry::{AttributeInfo, AttributeOwner, PartType, StorageType},
//!     session::simple_session,
//! };
//! use std::path::PathBuf;
//!
//! fn main() -> hapi_rs::Result<()> {
//!     let session = simple_session().unwrap();
//!     let hda = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../otls/hapi_geo.hda");
//!     let library = session.load_asset_file(&hda)?;
//!     let node = library.try_create_first()?;
//!     node.cook_blocking()?;
//!     let geometry = node.geometry()?.expect("SOP geometry");
//!     let part = geometry
//!         .partitions()?
//!         .into_iter()
//!         .find(|info| info.part_type() == PartType::Mesh)
//!         .expect("mesh part");
//!     let attr = geometry
//!         .get_attribute(part.part_id(), AttributeOwner::Point, "P")?
//!         .expect("P attribute");
//!     let positions = attr
//!         .downcast::<NumericAttr<f32>>()
//!         .expect("numeric P data");
//!     let values = positions.get(part.part_id())?;
//!     assert!(!values.is_empty());
//!
//!     let mut info = AttributeInfo::default();
//!     info.set_owner(AttributeOwner::Point);
//!     info.set_storage(StorageType::Float);
//!     info.set_tuple_size(1);
//!     info.set_count(part.point_count());
//!     let weights = geometry.add_numeric_attribute::<f32>("rs_weight", part.part_id(), info)?;
//!     let fill = vec![1.0f32; part.point_count() as usize];
//!     weights.set(part.part_id(), &fill)?;
//!     Ok(())
//! }
//! ```
//!
//! ## Parameters and UI metadata
//! [`parameter::Parameter`] is an enum that covers all `HAPI_ParmType` values, while
//! [`parameter::ParmBaseTrait`] exposes shared helpers (`name`, `label`, `size`, etc.). Matching on `Parameter`
//! gives you access to strongly typed wrappers such as [`parameter::FloatParameter`],
//! [`parameter::IntParameter`], [`parameter::StringParameter`], or button/toggle helpers. You can inspect menu
//! metadata, manipulate multiparms, attach expressions/animation curves, or call [`parameter::IntParameter::press_button`]
//! for script callbacks. [`parameter::ParmInfo`] mirrors `HAPI_ParmInfo` for low-level needs.
//!
//! Always cook an HDA before touching its parameters so that defaults are initialized. The snippet below uses
//! `../otls/hapi_parms.hda`, the same file leveraged by `lib/examples/parameters.rs`, and demonstrates how to
//! branch on parameter types. For more elaborate formatting of menu values see that example; it prints a table
//! of every parameter on the asset.
//!
//! ```rust
//! use hapi_rs::{
//!     parameter::{ParmBaseTrait, Parameter},
//!     session::simple_session,
//! };
//! use std::path::PathBuf;
//!
//! fn main() -> hapi_rs::Result<()> {
//!     let session = simple_session().unwrap();
//!     let hda = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../otls/hapi_parms.hda");
//!     let library = session.load_asset_file(&hda)?;
//!     let node = library.try_create_first()?;
//!     node.cook_blocking()?;
//!     if let Parameter::Float(color) = node.parameter("color")? {
//!         color.set_array([0.25, 0.5, 0.75])?;
//!         assert_eq!(color.size(), 3);
//!     }
//!     if let Parameter::Int(toggle) = node.parameter("toggle")? {
//!         let next = if toggle.get(0)? == 0 { 1 } else { 0 };
//!         toggle.set(0, next)?;
//!     }
//!     if let Parameter::String(path) = node.parameter("op_path")? {
//!         if let Some(target) = path.get_value_as_node()? {
//!             assert!(target.is_valid(&node.session)?);
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Error handling and diagnostics
//! Every public API returns [`Result`], an alias for `std::result::Result<T, [`HapiError`]>`.
//! `HapiError::Hapi` stores the [`errors::HapiResultCode`] plus an optional server message fetched through
//! [`session::Session::get_status_string`]. Additional context strings accumulate automatically when you call
//! `.context(...)` / `.with_context(...)` using the helper methods defined in the `errors` module, making it
//! easier to track which operation failed (for example `geometry::Geometry::get_attribute` annotates the
//! attribute name and owner).
//!
//! Other variants (null-byte, UTF-8, IO, internal) map directly to common Rust errors. When a cook happens in
//! threaded mode, methods return [`session::CookResult`] so you can inspect the cook-state message even if the
//! original call succeeded. `lib/examples/node_errors.rs` demonstrates how to read verbose cook logs and status
//! codes, while `lib/examples/materials.rs` shows how to propagate file IO errors during texture extraction.
//!
//! String-heavy APIs such as parameter values or attribute names rely on [`stringhandle::StringArray`] to batch
//! conversions and mirror `HAPI_StringHandle` semantics. Use [`session::Session::get_string`] or
//! [`session::Session::get_string_batch`] if you need direct access, or prefer the higher-level helpers where
//! possible.

pub mod asset;
pub mod attribute;
pub mod geometry;
pub mod material;
pub mod node;
pub mod cop;
pub mod parameter;
pub mod server;
pub mod session;
pub mod stringhandle;
pub mod volume;
pub mod pdg;
mod errors;
mod utils;
mod ffi;

pub use errors::{HapiError, HapiResult, HapiResultCode, Result};
pub use ffi::enums;
pub use ffi::raw;

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
