use std::path::Path;

fn main() {
    if let Ok(_) = std::env::var("DOCS_RS") {
        return;
    }
    let hfs = std::env::var("HFS").expect("HFS variable not set");
    let filename;
    let lib_dir;
    if cfg!(target_os = "macos") {
        filename = "HAPIL";
        let _lib_dir = Path::new(&hfs).parent().unwrap().join("Libraries");
        lib_dir = _lib_dir.to_string_lossy().to_string();
    } else if cfg!(target_os = "windows") {
        filename = "libHAPIL";
        lib_dir = format!("{}/custom/houdini/dsolib", hfs);
    } else {
        filename = "HAPIL";
        lib_dir = format!("{}/dsolib", hfs);
    }
    println!("cargo:rustc-link-search=native={}", lib_dir);
    println!("cargo:rustc-link-lib=dylib={}", filename);
}
