use serde::Deserialize;
use std::collections::HashMap;
use toml;

#[derive(Deserialize, Debug)]
pub struct CodeGenInfo {
    enums: HashMap<String, EnumOptions>,
    structs: HashMap<String, StructOption>,
}

#[derive(Deserialize, Debug)]
pub struct EnumOptions {
    name: String,
    field: i32,
}

#[derive(Deserialize, Debug)]
pub struct StructOption {
    name: String,
}

pub fn read_config() -> CodeGenInfo {
    let s = std::fs::read_to_string("bindgen_ext/codegen.toml").expect("Oops");
    let mut info: CodeGenInfo;
    match toml::from_str(&s) {
        Ok(c) => {
            info = c;
        }
        Err(e) => panic!(e),
    }
    info
}
