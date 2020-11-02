use std::path::Path;

fn warning(msg: impl AsRef<str>) {
    println!("cargo:warning={}", msg.as_ref());
}
pub fn write_extension(bindings: &Path, extension: &Path) {
    warning(bindings.to_string_lossy());
    println!("His")
}

fn main() {

}