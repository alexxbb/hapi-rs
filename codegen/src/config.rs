use crate::helpers;
use crate::helpers::StripMode;
use serde::Deserialize;
use std::collections::HashMap;
use toml;

#[derive(Deserialize, Debug)]
pub struct CodeGenConfig {
    enums: HashMap<String, EnumOptions>,
    structs: HashMap<String, StructOptions>,
}

impl CodeGenConfig {
    pub fn enum_opt(&self, name: impl AsRef<str>) -> Option<EnumOptions> {
        self.enums.get(name.as_ref()).map(|o| o.clone())
    }
    pub fn struct_opt(&self, name: impl AsRef<str>) -> Option<StructOptions> {
        self.structs.get(name.as_ref()).map(|o| o.clone())
    }

    pub fn new_name<'a>(&'a self, ffi_name: &'a str) -> &'a str {
        let rename = if let Some(o) = self.enums.get(ffi_name) {
            Some(o.rename.as_ref())
        } else if let Some(o) = self.structs.get(ffi_name) {
            Some(o.rename.as_ref())
        } else {
            None
        };
        match rename {
            Some(n) => new_name(n, ffi_name),
            None => ffi_name
        }

    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct StructOptions {
    pub rename: String,
    #[serde(deserialize_with = "mode")]
    pub mode: StripMode,
    pub derive: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EnumOptions {
    pub rename: String,
    #[serde(deserialize_with = "mode")]
    pub mode: StripMode,
    pub bitflag: Option<bool>,
}

pub fn new_name<'a>(rename: &'a str, name: &'a str) -> &'a str {
    match rename {
        "auto" => name.strip_prefix("HAPI_").expect("Not a HAPI enum"),
        n => n,
    }
}

impl StructOptions{
    pub fn new_name<'a>(&'a self, name: &'a str) -> &'a str {
        new_name(&self.rename, name)
    }
}
impl EnumOptions{
    pub fn new_name<'a>(&'a self, name: &'a str) -> &'a str {
        new_name(&self.rename, name)
    }
}

fn mode<'de, D>(d: D) -> Result<StripMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let num = i32::deserialize(d)?;
    Ok(if num < 0 {
        StripMode::KeepTail(num.abs() as u8)
    } else {
        StripMode::StripFront(num as u8)
    })
}

pub fn read_config(path: &str) -> CodeGenConfig {
    let s = std::fs::read_to_string(path).expect("Oops");
    let mut info: CodeGenConfig;
    match toml::from_str(&s) {
        Ok(c) => {
            info = c;
        }
        Err(e) => panic!(e.to_string()),
    }
    info
}
