#![allow(dead_code, unused)]
use regex_lite::{Regex, RegexBuilder};
use std::collections::HashSet;
use std::error::Error;
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
        "HAPI_GetWorkitemResultInfo",
        "HAPI_ParmInfo_GetIntValueCount",
        "HAPI_ParmInfo_GetFloatValueCount",
        "HAPI_ParmInfo_GetStringValueCount",
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

#[derive(Debug, Eq, Ord, PartialOrd)]
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

fn wrapped_rs_function_names() -> Result<HashSet<Item>, Box<dyn Error>> {
    let rx1 = Regex::new(r#"raw::(HAPI\w+)\(?"#).unwrap();
    let rx2 = Regex::new(r#"\[(HAPI\w+)\]"#).unwrap();
    let rx3 = Regex::new(r#".*raw::(HAPI\w+)\("#).unwrap();

    let mut set = HashSet::new();

    let text = std::fs::read_to_string("../../src/ffi/functions.rs").expect("ffi/functions.rs");

    let it1 = rx1.captures_iter(&text).map(|c| Item(c[1].to_string()));
    let it2 = rx2.captures_iter(&text).map(|c| Item(c[1].to_string()));
    let ffi_functions = it1.chain(it2);
    set.extend(ffi_functions);

    for entry in glob::glob("../../src/attribute/**/*.rs").unwrap() {
        let path = entry.expect("failed to read glob entry");
        let text = std::fs::read_to_string(&path)?;
        let it1 = rx2.captures_iter(&text).map(|c| Item(c[1].to_string()));
        let it2 = rx3.captures_iter(&text).map(|c| Item(c[1].to_string()));
        let attribute_bindings = it1.chain(it2);
        set.extend(attribute_bindings);
    }
    Ok(set)
}

fn main() {
    let mut ffi_functions = Vec::from_iter(raw_hapi_function_names().into_iter());
    ffi_functions.sort();
    let rs = wrapped_rs_function_names().unwrap();
    let mut num_missed: i32 = 0;
    for func in ffi_functions.iter() {
        if !rs.contains(func) {
            println!("Missing {func:?}");
            num_missed += 1;
        }
    }
    println!("Missed {} functions", num_missed);
}
