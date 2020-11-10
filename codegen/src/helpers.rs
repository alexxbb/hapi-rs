use log::{error, warn};
use serde::Deserialize;
use heck;
use heck::SnakeCase;

#[derive(Deserialize, Debug, Copy, Clone)]
pub enum StripMode {
    StripFront(u8), // 1: FOO_BAR_ZOO => BAR_ZOO
    KeepTail(u8),   // 1: FOO_BAR_ZOO => ZOO
}

#[derive(Debug, Copy, Clone)]
pub enum CaseMode {
    EnumVariant,
    StructField,
    Function,
    Item,
}

pub fn strip_long_name(name: &str, mode: StripMode) -> &str {
    let mut iter = name.match_indices('_');
    let elem = match mode {
        StripMode::KeepTail(i) => {
            iter.nth_back((i - 1) as usize)
        }
        StripMode::StripFront(i) => {
            iter.nth((i - 1) as usize)
        },
    };
    let new_name = match elem {
        Some((idx, _)) => &name[idx + 1..name.len()],
            None => {
                warn!("Not enough length: {}", name);
                name
            }
        };
    match new_name.chars().take(1).next() {
        None => {
            error!("Empty string {}", name);
            name
        }
        Some(c) if c.is_digit(10) => {
          strip_long_name(name, match mode {
              StripMode::StripFront(v) => StripMode::StripFront(v + 1),
              StripMode::KeepTail(v) => StripMode::KeepTail(v + 1),
          })
        }
        Some(_) => new_name
    }
}

pub fn change_case(name: &str, mode: CaseMode) -> String {
    use CaseMode::*;
    match mode {
        EnumVariant | Item => {
            use heck::CamelCase;
            name.to_camel_case()
        },
        Function => {
            use heck::SnakeCase;
            name.to_snake_case()
        }
        StructField => {
            use heck::SnakeCase;
            name.to_snake_case()
        }
    }
}
