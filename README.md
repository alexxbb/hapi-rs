# Rust bindings to Houdini Engine API

[![Cargo](https://img.shields.io/crates/v/hapi-rs.svg)](https://crates.io/crates/hapi-rs)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)

SideFX Houdini Meets Rust!

[SideFx Houdini](https://www.sidefx.com/) is a world leading software for creating stunning visual effects for movies
and games. Apart from the main graphical interface written in C++ and Python, Houdini also provides a C interface
called [Houdini Engine](https://www.sidefx.com/products/houdini-engine/) or HAPI for short. Its goal is to bring the
power of Houdini to other DCCs (Digital Content Creation) software and game engines.

This crate aims to provide idiomatic Rust interface to Houdini Engine and is built on top
of [hapi-sys](https://crates.io/crates/hapi-sys).

> :exclamation: A valid **commercial** Houdini Engine license is required to use this crate

# Example

```rust
use hapi_rs::Result;
use hapi_rs::session::quick_session;
use hapi_rs::parameter::*;

fn main() -> Result<()> {
    // Start a standalone engine process
    let session = quick_session()?;
    // Load a Houdini Asset and create a node
    session.load_asset_file("otls/hapi_geo.hda")?;
    let node = session.create_node("Object/hapi_geo", None, None)?;
    // Set the "scale" parameter
    if let Parameter::Float(parm) = node.parameter("scale")? {
        parm.set_value(&[3.0])?;
        node.cook(None)?;
    }
    // Get a reference to the node's internal geometry
    let geometry = node.geometry()?.expect("geometry");
    // Save it to one of the supported geometry formats
    geometry.save_to_file("/tmp/output.fbx")?;
    Ok(())
}
```

# Building

Check the documentation [building section](https://docs.rs/hapi-rs/latest/hapi-rs/#building-and-running)

[HAPI C Documentation](https://www.sidefx.com/docs/hengine/)

[More Examples](https://github.com/alexxbb/hapi-rs/tree/main/examples)
