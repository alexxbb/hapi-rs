#![allow(dead_code, unused)]
use std::collections::HashSet;
use std::path::Path;

fn raw_hapi_function_names() -> HashSet<String> {
    const IGNORE_SUFFIX: &[&str] = &["_IsString", "_Create", "_Init"];
    let raw = Path::new("../../src/ffi/bindings.rs");
    let text = std::fs::read_to_string(&raw).expect("bindings.rs");
    let rx = regex_lite::Regex::new(r#"pub fn (HAPI\w+)\("#).unwrap();
    let matches: HashSet<_> = rx
        .captures_iter(&text)
        .filter_map(|m| {
            let name = &m[1];
            let skip = IGNORE_SUFFIX.iter().any(|suf| name.ends_with(suf));
            if skip {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect();
    matches
}

fn wrapped_rs_function_names() -> HashSet<String> {
    let rx1 = regex_lite::Regex::new(r#"raw::(HAPI\w+)\("#).unwrap();
    let func = std::fs::read_to_string("../../src/ffi/functions.rs").expect("functions.rs");
    let it1 = rx1.captures_iter(&func).map(|c| c[1].to_string());

    let aatr = std::fs::read_to_string("../../src/attribute/bindings.rs").expect("functions.rs");
    let rx2 = regex_lite::Regex::new(r#"\[(HAPI\w+)\]"#).unwrap();
    let it2 = rx2.captures_iter(&aatr).map(|c| c[1].to_string());

    let rx3 = regex_lite::Regex::new(r#"raw::(HAPI\w+)"#).unwrap();
    let it3 = rx2.captures_iter(&aatr).map(|c| c[1].to_string());
    HashSet::from_iter(it1.chain(it2).chain(it3))
}

fn main() {
    let raw = raw_hapi_function_names();
    let rs = wrapped_rs_function_names();
    let v: Vec<_> = rs.iter().collect();
    for r in raw.iter() {
        if !rs.contains(r) {
            println!("Missing {r}");
        }
    }
}
