use log::{error, warn};
use serde::Deserialize;

#[derive(Deserialize, Debug, Copy, Clone)]
pub enum Mode {
    StripFront(u8), // 1: FOO_BAR_ZOO => BAR_ZOO
    KeepTail(u8),   // 1: FOO_BAR_ZOO => ZOO
}

pub fn strip_long_name(name: &str, mode: Mode) -> &str {
    let mut iter = name.match_indices('_');
    let elem = match mode {
        Mode::KeepTail(i) => {
            iter.nth_back((i - 1) as usize)
        }
        Mode::StripFront(i) => {
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
              Mode::StripFront(v) => Mode::StripFront(v + 1),
              Mode::KeepTail(v) => Mode::KeepTail(v + 1),
          })
        }
        Some(_) => new_name
    }
}
