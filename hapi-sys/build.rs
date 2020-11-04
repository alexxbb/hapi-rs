use std::env;
use std::path::PathBuf;
// #[path="build_ext.rs"] mod ext;

#[cfg(target_os = "macos")]
mod paths {
    pub static HAPI_INCLUDE: &str = "/Applications/Houdini/Houdini18.5.351/Frameworks/Houdini.framework/Versions/Current/Resources/toolkit/include/HAPI";
    pub static LIBS: &str = "/Applications/Houdini/Houdini18.5.351/Frameworks/Houdini.framework/Versions/Current/Libraries/";
}

#[cfg(target_os = "linux")]
mod paths {
    pub static HAPI_INCLUDE: &str = "/net/apps/rhel7/houdini/hfs18.0.530/toolkit/include/HAPI/";
    pub static LIBS: &str = "/net/apps/rhel7/houdini/hfs18.0.530/dsolib";
}

use paths::*;

fn main() {
    if cfg!(target_os = "linux") {
        std::env::set_var(
            "LIBCLANG_PATH",
            "/shots/spi/home/software/packages/llvm/11.0.0/gcc-6.3/lib",
        );
    }
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I/{}", HAPI_INCLUDE))
        .default_enum_style("rust_non_exhaustive".parse().unwrap())
        .prepend_enum_name(false)
        .generate_comments(false)
        .layout_tests(false)
        .generate()
        .expect("Oops");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindings_rs = out_path.join("bindings.rs");
    // let extension_rs = out_path.join("extension.rs");
    bindings
        .write_to_file(&bindings_rs)
        .expect("Couldn't write bindings!");
    // ext::write_extension(&bindings_rs, &extension_rs);
    std::fs::copy(bindings_rs, "/tmp/hapi.rs");
    println!("cargo:rustc-link-search={}", LIBS);
    println!("cargo:rustc-link-lib=dylib=HAPI");
}
