use bindgen;
use std::env;
use std::path::{PathBuf};

static HAPI_INCLUDE: &str = "/Applications/Houdini/Houdini18.5.351/Frameworks/Houdini.framework/Versions/Current/Resources/toolkit/include/HAPI";

fn main() {
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I/{}", HAPI_INCLUDE))
        .generate().expect("Oops");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}