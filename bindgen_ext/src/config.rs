use serde::Deserialize;
use std::collections::HashMap;
use toml;

#[derive(Deserialize, Debug)]
pub struct CodeGenInfo {
    enums: HashMap<String, EnumOptions>,
    structs: HashMap<String, StructOption>,
}

impl CodeGenInfo {
    pub fn enum_opt(&self, name: impl AsRef<str>) -> Option<EnumOptions> {
        self.enums.get(name.as_ref()).map(|o|o.clone())
    }
}


#[derive(Deserialize, Debug, Clone)]
pub struct EnumOptions {
    pub rename: String,
    pub variant: i32,
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
