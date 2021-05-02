use std::path::Path;

fn main() {
    let hfs = std::env::var("HFS").expect("HFS variable not set");
    println!("cargo:rustc-link-lib=dylib=HAPIL");
    if cfg!(target_os = "macos") {
        let lib_dir = Path::new(&hfs).parent().unwrap().join("Libraries");
        println!(
            "cargo:rustc-link-search=native={}",
            lib_dir.to_string_lossy()
        );
    } else {
        println!("cargo:rustc-link-search=native={}/dsolib", hfs);
    }
}
