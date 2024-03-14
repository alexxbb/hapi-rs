#![allow(dead_code, unused)]
use regex_lite::{Regex, RegexBuilder};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::Path;

fn raw_hapi_function_names() -> HashSet<Item> {
    const IGNORE_SUFFIX: &[&str] = &[
        "_IsString",
        "_IsNonValue",
        "_IsFloat",
        "_IsInt",
        "_AreEqual",
        "_IsPath",
        "_IsNode",
        "_Create",
        "_Init",
        "HAPI_CreateCustomSession",
        "HAPI_SetCustomString",
        "HAPI_RemoveCustomString",
        "HAPI_GetHandleInfo",
        "HAPI_BindCustomImplementation",
        "HAPI_GetImageFilePath",
        "HAPI_GetHandleBindingInfo",
        "HAPI_GetWorkitemResultInfo"
    ];
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
                Some(Item(name.to_string()))
            }
        })
        .collect();
    matches
}

#[derive(Debug, Eq)]
struct Item(String);

impl PartialEq for Item {
    fn eq(&self, other: &Item) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

impl Hash for Item {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.0.to_lowercase().hash(hasher)
    }
}

fn wrapped_rs_function_names() -> HashSet<Item> {
    let rx1 = Regex::new(r#"raw::(HAPI\w+)\(?"#).unwrap();
    let rx2 = Regex::new(r#"\[(HAPI\w+)\]"#).unwrap();
    let rx3 = Regex::new(r#".*raw::(HAPI\w+)\("#).unwrap();

    let text = std::fs::read_to_string("../../src/ffi/functions.rs").expect("ffi/functions.rs");
    let it1 = rx1.captures_iter(&text).map(|c| Item(c[1].to_string()));
    let it2 = rx2.captures_iter(&text).map(|c| Item(c[1].to_string()));
    let ffi_functions = it1.chain(it2);

    let text =
        std::fs::read_to_string("../../src/attribute/bindings.rs").expect("attribute/bindings.rs");
    let it1 = rx2.captures_iter(&text).map(|c| Item(c[1].to_string()));
    let it2 = rx3.captures_iter(&text).map(|c| Item(c[1].to_string()));
    let attribute_bindings = it1.chain(it2);

    HashSet::from_iter(ffi_functions.chain(attribute_bindings))
}

fn main() {
    let raw = raw_hapi_function_names();
    let rs = wrapped_rs_function_names();
    for r in raw.iter() {
        if !rs.contains(r) {
            println!("Missing {r:?}");
        }
    }
}
