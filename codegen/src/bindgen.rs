use bindgen::Builder;
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};

#[cfg(target_os = "macos")]
mod paths {
    pub static HAPI_INCLUDE: &str = "/Applications/Houdini/Houdini18.0.597/Frameworks/Houdini.framework/Versions/Current/Resources/toolkit/include/HAPI";
    pub static LIBS: &str = "/Applications/Houdini/Houdini18.0.597/Frameworks/Houdini.framework/Versions/Current/Libraries/";
}

#[cfg(target_os = "linux")]
mod paths {
    pub static HAPI_INCLUDE: &str = "/net/apps/rhel7/houdini/hfs18.0.530/toolkit/include/HAPI/";
    pub static LIBS: &str = "/net/apps/rhel7/houdini/hfs18.0.530/dsolib";
}

fn main() {
    if cfg!(target_os = "linux") {
        std::env::set_var(
            "LIBCLANG_PATH",
            "/shots/spi/home/software/packages/llvm/11.0.0/gcc-6.3/lib",
        );
    }
}

pub fn run_bindgen(incl: &str, header: &str, outdir: &str) -> Result<()> {
    let bindings = bindgen::Builder::default()
        .header(header)
        .clang_arg(format!("-I/{}", incl))
        .default_enum_style("rust_non_exhaustive".parse().unwrap())
        .prepend_enum_name(false)
        .generate_comments(false)
        .derive_copy(false)
        .derive_debug(true)
        .derive_hash(false)
        .derive_eq(false)
        .derive_partialeq(false)
        .disable_name_namespacing()
        .layout_tests(false)
        .generate().map_err(|_|anyhow!("Bindgen generate failed"))?;

    let out_path = PathBuf::from(outdir);
    let bindings_rs = out_path.join("bindings.rs");
    bindings.write_to_file(&bindings_rs)?;
    Ok(())
    // println!("cargo:rustc-link-search={}", LIBS);
    // println!("cargo:rustc-link-lib=dylib=HAPI");
}
