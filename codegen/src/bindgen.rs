use crate::config::CodeGenConfig;
use anyhow::{anyhow, Result};
use bindgen::callbacks::{EnumVariantValue, ParseCallbacks};

use std::rc::Rc;
use crate::helpers::*;

#[cfg(target_os = "macos")]
mod paths {
    pub static HAPI_INCLUDE: &str = "/Applications/Houdini/Current/Frameworks/Houdini.framework/Versions/Current/Resources/toolkit/include/HAPI";
    pub static LIBS: &str = "/Applications/Houdini/Current/Frameworks/Houdini.framework/Versions/Current/Libraries/";
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
struct Rustifier {
    cc: Rc<CodeGenConfig>,
}

impl Rustifier {
    pub fn new(cc: Rc<CodeGenConfig>) -> Rustifier {
        Rustifier { cc }
    }
}

impl ParseCallbacks for Rustifier {
    fn enum_variant_name(
        &self,
        _enum_name: Option<&str>,
        _variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<String> {
        if let Some(name) = _enum_name {
            let name = name.strip_prefix("enum ").expect("Not enum?");
            match self.cc.enum_opt(name) {
                Some(opt) => {
                    let striped = strip_long_name(_variant_name, opt.mode);
                    let new = change_case(striped, CaseMode::EnumVariant);
                    Some(new)
                }
                None => {
                    println!("Missing enum config {}", name);
                    None
                }
            }

        } else {
            None
        }
    }

    fn item_name(&self, _item_name: &str) -> Option<String> {
        if let Some(en) = self.cc.enum_opt(_item_name) {
            return Some(en.new_name(_item_name).to_string())
        }
        None
    }
}

pub fn run_bindgen(incl: &str, header: &str, cgc: Rc<CodeGenConfig>) -> Result<String> {
    let callbacks = Box::new(Rustifier::new(cgc));
    let bindings = bindgen::Builder::default()
        .header(header)
        .clang_arg(format!("-I/{}", incl))
        .default_enum_style("rust_non_exhaustive".parse().unwrap())
        .bitfield_enum("NodeType")
        .bitfield_enum("NodeFlags")
        .bitfield_enum("ErrorCode")
        .prepend_enum_name(false)
        .generate_comments(false)
        // .raw_line("use strum_macros::AsRefStr;")
        .derive_copy(true)
        .derive_debug(true)
        .derive_hash(false)
        .derive_eq(false)
        .derive_partialeq(false)
        .disable_name_namespacing()
        .rustfmt_bindings(false)
        .layout_tests(false)
        .parse_callbacks(callbacks)
        .generate()
        .map_err(|_| anyhow!("Bindgen generate failed"))?;

    Ok(bindings.to_string())
}
