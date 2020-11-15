#!/bin/zsh

incl="/Applications/Houdini/Houdini18.0.597/Frameworks/Houdini.framework/Versions/Current/Resources/toolkit/include/HAPI"

cargo run --package codegen -- --include $incl --outdir hapi-rs/src/auto --config codegen/codegen.toml --wrapper codegen/wrapper.h