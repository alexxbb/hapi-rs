fn main() {
    println!("cargo:rustc-link-lib=dylib=HAPIL");
    if cfg!(darwin){
        println!("cargo:rustc-link-search=native=/Applications/Houdini/Houdini18.0.597/Frameworks/Houdini.framework/Versions/Current/Libraries");
    } else {
        println!("cargo:rustc-link-search=native=/net/apps/rhel7/houdini/hfs18.0.597/dsolib");
    }
}
