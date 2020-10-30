use bindgen;
use std::env;
use std::path::PathBuf;

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
        .layout_tests(false)
        .generate()
        .expect("Oops");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
    println!("cargo:rustc-link-search={}", LIBS);
    println!("cargo:rustc-link-lib=dylib=HAPI");
}
