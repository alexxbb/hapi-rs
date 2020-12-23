
use crate::helpers::StripMode;
use serde::Deserialize;
use std::collections::HashMap;
use toml;

#[derive(Deserialize, Debug)]
pub struct CodeGenConfig {
    enums: HashMap<String, EnumOptions>,
}

impl CodeGenConfig {
    pub fn enum_opt(&self, name: impl AsRef<str>) -> Option<EnumOptions> {
        self.enums.get(name.as_ref()).map(|o| o.clone())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct EnumOptions {
    pub rename: String,
    #[serde(deserialize_with = "mode")]
    pub mode: StripMode,
    pub bitflag: Option<bool>,
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

impl EnumOptions {
    pub fn new_name<'a>(&'a self, name: &'a str) -> &'a str {
        match self.rename.as_ref() {
            "auto" => name.strip_prefix("HAPI_").expect("Not a HAPI enum"),
            n => n,
        }
    }
}

pub fn read_config(path: &str) -> CodeGenConfig {
    let s = std::fs::read_to_string(path).expect("Oops");
    let info: CodeGenConfig;
    match toml::from_str(&s) {
        Ok(c) => {
            info = c;
        }
        Err(e) => panic!(e.to_string()),
    }
    info
}
