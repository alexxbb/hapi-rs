use crate::enums;
use crate::bitflags;
use crate::structs;
use crate::config::CodeGenInfo;
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use std::io::Write;

fn rustfmt(path: &str) -> Result<()> {
    std::process::Command::new("rustfmt")
        .arg(path)
        .status()?;
    Ok(())
}

pub fn write_extension(outdir: &str, cg: CodeGenInfo) -> Result<()> {
    let bindings_rs = Path::new(outdir).join("bindings.rs");
    let rusty_rs = Path::new(outdir).join("rusty.rs");
    let mut auto_f = std::fs::File::create(&rusty_rs)?;
    assert!(bindings_rs.exists());
    let source = std::fs::read_to_string(bindings_rs)?;
    let tree: syn::File = syn::parse_file(&source)?;
    let enum_tokens = enums::generate_enums(&tree.items, &cg);
    let bitflag_tokens = bitflags::generate_bitflags(&tree.items, &cg);
    let struct_tokens = structs::generate_structs(&tree.items, &cg);
    auto_f.write_all(b"/* Auto generated hapi-codegen */\n");
    auto_f.write_all(b"use crate::auto::bindings as ffi;\n");
    for e in enum_tokens.iter()
        .chain(bitflag_tokens.iter())
        .chain(struct_tokens.iter()){
        auto_f.write_all(e.to_string().as_bytes());
    }
    rustfmt(&rusty_rs.to_string_lossy())?;
    Ok(())
}
