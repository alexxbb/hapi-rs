use crate::helpers;
use serde::Deserialize;
use std::collections::HashMap;
use toml;
use crate::helpers::Mode;

#[derive(Deserialize, Debug)]
pub struct CodeGenInfo {
    enums: HashMap<String, EnumOptions>,
    structs: HashMap<String, StructOption>,
}

impl CodeGenInfo {
    pub fn enum_opt(&self, name: impl AsRef<str>) -> Option<EnumOptions> {
        self.enums.get(name.as_ref()).map(|o| o.clone())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct EnumOptions {
    pub rename: String,
    #[serde(deserialize_with = "mode")]
    pub mode: helpers::Mode,
}

fn mode<'de, D>(d: D) -> Result<Mode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let num = i32::deserialize(d)?;
    Ok(if num < 0 {
        helpers::Mode::KeepTail(num.abs() as u8)
    }else {
        helpers::Mode::StripFront(num as u8)
    })
}

#[derive(Deserialize, Debug)]
pub struct StructOption {
    name: String,
}

pub fn read_config(path: &str) -> CodeGenInfo {
    let s = std::fs::read_to_string(path).expect("Oops");
    let mut info: CodeGenInfo;
    match toml::from_str(&s) {
        Ok(c) => {
            info = c;
        }
        Err(e) => panic!(e)
    }
    info
}
