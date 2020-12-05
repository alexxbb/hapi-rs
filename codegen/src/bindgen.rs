use crate::helpers;
use anyhow::{anyhow, Result};
use bindgen::callbacks::{EnumVariantValue, ParseCallbacks};
use bindgen::Builder;
use std::path::{Path, PathBuf};

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

#[derive(Debug)]
struct Renamer {}

impl ParseCallbacks for Renamer {
    fn enum_variant_name(
        &self,
        _enum_name: Option<&str>,
        _original_variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<String> {
        match _enum_name {
            Some(e) if e == "enum HAPI_NodeType" => {
                let new = helpers::strip_long_name(
                    _original_variant_name,
                    helpers::StripMode::StripFront(2),
                );
                Some(helpers::change_case(new, helpers::CaseMode::EnumVariant))
            }
            _ => None,
        }
    }

    fn item_name(&self, _original_item_name: &str) -> Option<String> {
        if _original_item_name == "HAPI_NodeType" {
            return Some("NodeType".to_string());
        } else if _original_item_name == "HAPI_NodeTypeBits" {
            return Some("NodeTypeBits".to_string());
        }
        None
    }
}

pub fn run_bindgen(incl: &str, header: &str, outdir: &str) -> Result<()> {
    let bindings = bindgen::Builder::default()
        .header(header)
        .clang_arg(format!("-I/{}", incl))
        .default_enum_style("rust_non_exhaustive".parse().unwrap())
        // .constified_enum_module("HAPI_NodeType")
        .bitfield_enum("NodeType")
        .constified_enum_module("HAPI_NodeFlags")
        .constified_enum_module("HAPI_ErrorCode")
        .prepend_enum_name(false)
        .generate_comments(false)
        .derive_copy(true)
        .derive_debug(true)
        .derive_hash(false)
        .derive_eq(false)
        .derive_partialeq(false)
        .disable_name_namespacing()
        .layout_tests(false)
        .parse_callbacks(Box::new(Renamer {}))
        .generate()
        .map_err(|_| anyhow!("Bindgen generate failed"))?;

    let out_path = PathBuf::from(outdir);
    let bindings_rs = out_path.join("bindings.rs");
    bindings.write_to_file(&bindings_rs)?;
    Ok(())
}
