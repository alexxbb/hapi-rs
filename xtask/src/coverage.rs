use regex_lite::Regex;
use std::collections::HashSet;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Debug, Eq, Ord, PartialOrd)]
struct Item(String);

impl PartialEq for Item {
    fn eq(&self, other: &Item) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Hash for Item {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.0.to_ascii_lowercase().hash(hasher)
    }
}

pub fn run(workspace_root: &Path, _args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut ffi_functions = Vec::from_iter(raw_hapi_function_names(workspace_root));
    ffi_functions.sort();
    let wrapped = wrapped_rs_function_names(workspace_root)?;
    let mut num_missed = 0;
    for func in &ffi_functions {
        if !wrapped.contains(func) {
            println!("Missing {func:?}");
            num_missed += 1;
        }
    }
    println!("Missed {num_missed} functions");
    Ok(())
}

fn source_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("lib/src")
}

fn raw_hapi_function_names(workspace_root: &Path) -> HashSet<Item> {
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
        "HAPI_GetHandleInfo",
        "HAPI_BindCustomImplementation",
        "HAPI_GetImageFilePath",
        "HAPI_GetHandleBindingInfo",
        "HAPI_GetWorkitemResultInfo",
        "HAPI_ParmInfo_GetIntValueCount",
        "HAPI_ParmInfo_GetFloatValueCount",
        "HAPI_ParmInfo_GetStringValueCount",
    ];

    let raw = source_dir(workspace_root).join("ffi/bindings.rs");
    let text = std::fs::read_to_string(raw).expect("bindings.rs");
    let rx = Regex::new(r#"pub fn (HAPI\w+)\("#).expect("valid regex");
    rx.captures_iter(&text)
        .filter_map(|m| {
            let name = &m[1];
            if IGNORE_SUFFIX.iter().any(|suffix| name.ends_with(suffix)) {
                None
            } else {
                Some(Item(name.to_string()))
            }
        })
        .collect()
}

fn wrapped_rs_function_names(workspace_root: &Path) -> Result<HashSet<Item>, Box<dyn Error>> {
    let rx1 = Regex::new(r#"raw::(HAPI\w+)\(?"#)?;
    let rx2 = Regex::new(r#"\[(HAPI\w+)\]"#)?;
    let rx3 = Regex::new(r#".*raw::(HAPI\w+)\("#)?;

    let source = source_dir(workspace_root);
    let mut set = HashSet::new();

    let text = std::fs::read_to_string(source.join("ffi/functions.rs"))?;
    let ffi_functions = rx1
        .captures_iter(&text)
        .map(|c| Item(c[1].to_string()))
        .chain(rx2.captures_iter(&text).map(|c| Item(c[1].to_string())));
    set.extend(ffi_functions);

    let pattern = source.join("attribute/**/*.rs");
    for entry in glob::glob(pattern.to_string_lossy().as_ref())? {
        let path = entry?;
        let text = std::fs::read_to_string(path)?;
        let attribute_bindings = rx2
            .captures_iter(&text)
            .map(|c| Item(c[1].to_string()))
            .chain(rx3.captures_iter(&text).map(|c| Item(c[1].to_string())));
        set.extend(attribute_bindings);
    }

    Ok(set)
}
