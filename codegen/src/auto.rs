use crate::enums;
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

pub fn write_auto(outdir: &str, cg: CodeGenInfo) -> Result<()> {
    let bindings_rs = Path::new(outdir).join("bindings.rs");
    let auto_rs = Path::new(outdir).join("auto.rs");
    let mut auto_f = std::fs::File::create(&auto_rs)?;
    assert!(bindings_rs.exists());
    let source = std::fs::read_to_string(bindings_rs)?;
    let tree: syn::File = syn::parse_file(&source)?;
    let enum_tokens = enums::generate_enums(&tree.items, &cg);
    for e in &enum_tokens {
        auto_f.write_all(e.to_string().as_bytes());
    }
    rustfmt(&auto_rs.to_string_lossy())?;
    Ok(())
}
