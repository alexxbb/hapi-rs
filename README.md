# Rust bindings to Houdini Engine API

[![Cargo](https://img.shields.io/crates/v/hapi-rs.svg)](https://crates.io/crates/hapi-rs)
[![Documentation](https://docs.rs/hapi-rs/badge.svg)](https://docs.rs/hapi-rs)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)

SideFX Houdini Meets Rust!

[SideFx Houdini](https://www.sidefx.com/) is world leading software for creating stunning visual effects for movies
and games. Apart from the main graphical interface written in C++ and Python, Houdini also provides a C interface
called [Houdini Engine](https://www.sidefx.com/products/houdini-engine/) or HAPI for short. Its goal is to bring the
power of Houdini to other DCCs (Digital Content Creation) software and game engines.

> :exclamation: A valid **commercial** Houdini Engine license is required to use this crate

# HDA Viewer Demo
https://user-images.githubusercontent.com/10694389/227816361-b9ffa083-7932-40e4-afda-3cb400126eb5.mp4

[Introduction Video On Youtube](https://youtu.be/D4-vXCzde28?si=WkKhq_N-gDDgtNSC)

# Example

```rust
use hapi_rs::Result;
use hapi_rs::server::ServerOptions;
use hapi_rs::session::{SessionOptions, new_thrift_session};
use hapi_rs::parameter::*;

fn main() -> Result<()> {
    // Start a standalone engine process
    let session =
        new_thrift_session(SessionOptions::default(), ServerOptions::shared_memory())?;
    // Load a Houdini Asset and create a node
    let lib = session.load_asset_file("otls/hapi_geo.hda")?;
    let node = lib.create_asset_for_node("Object/hapi_geo", None)?;
    // Set the "scale" parameter
    if let Parameter::Float(parm) = node.parameter("scale")? {
        parm.set(0, 3.0)?;
        node.cook(None)?;
    }
    // Get a reference to the node's internal geometry
    let geometry = node.geometry()?.expect("geometry");
    // Save it to one of the supported geometry formats
    geometry.save_to_file("/tmp/output.fbx")?;
    Ok(())
}
```

# Supported Houdini versions

The crate version matches the supported Houdini MAJOR.MINOR version i.e. `21.0.0` is built work with `Houdini 21.0`
Mixing and matching different versions of Houdini and this crate is not guranteed to work.

# Building

Check the documentation [building section](https://docs.rs/hapi-rs/latest/hapi_rs/#building-and-running)

[HAPI C Documentation](https://www.sidefx.com/docs/hengine/)

[More Examples](https://github.com/alexxbb/hapi-rs/tree/main/lib/examples)

