use std::{num::ParseIntError, path::Path};

fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }
    if std::env::var("CI").is_ok() {
        println!("cargo:warning=Skipping build script in CI");
        return;
    }
    let hfs = std::env::var("HFS").expect("HFS variable not set");
    let version_file = Path::new(&hfs).join("toolkit").join("hdk_api_version.txt");

    let hdk_api_version = std::fs::read_to_string(version_file)
        .expect("Failed to read hdk_api_version.txt")
        .trim()
        .to_owned();

    assert!(
        hdk_api_version.len() == 8,
        "Invalid version string in hdk_api_version.txt: {hdk_api_version}"
    );

    let (hdk_major, hdk_minor, _) = {
        let version_number: i32 = hdk_api_version
            .parse()
            .expect("Failed to parse version string into i32");
        let major = version_number / 1_000_000;
        let minor = (version_number / 10_000) % 100;
        let build = version_number % 10_000;
        (major, minor, build)
    };
    // Use the CARGO_PKG_VERSION environment variable to get the current crate version
    let crate_version =
        std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
    let crate_version_parts = crate_version
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<i32>, ParseIntError>>()
        .expect("Failed to parse crate version string into components");

    assert!(
        crate_version_parts.len() >= 3,
        "Invalid crate version string in Cargo.toml, expected at least 3 components"
    );

    let (crate_major, crate_minor) = (crate_version_parts[0], crate_version_parts[1]);
    assert!(
        !(crate_major != hdk_major || crate_minor != hdk_minor),
        "Version mismatch: crate version {crate_version} does not match runtime Houdini version pointed by HFS: {hfs}"
    );

    let filename;
    let lib_dir;
    if cfg!(target_os = "macos") {
        filename = "HAPIL";
        let macos_lib_dir = Path::new(&hfs).parent().unwrap().join("Libraries");
        lib_dir = macos_lib_dir.to_string_lossy().to_string();
    } else if cfg!(target_os = "windows") {
        filename = "libHAPIL";
        lib_dir = format!("{hfs}/custom/houdini/dsolib");
    } else {
        filename = "HAPIL";
        lib_dir = format!("{hfs}/dsolib");
    }
    println!("cargo:rustc-link-search=native={lib_dir}");
    println!("cargo:rustc-link-lib=dylib={filename}");
    println!("cargo:rerun-if-env-changed=HFS");
}
