fn main() {
    println!("cargo:rustc-link-lib=dylib=HAPI");
    println!("cargo:rustc-link-search=native=/Applications/Houdini/Houdini18.0.597/Frameworks/Houdini.framework/Versions/Current/Libraries");
}