#![allow(dead_code)]
#![allow(unused)]

use anyhow::Context;
use argh::FromArgs;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env::var;
use std::path::{Path, PathBuf};

use bindgen::callbacks::{EnumVariantValue, ParseCallbacks};
use once_cell::sync::Lazy;

#[derive(Debug, Copy, Clone)]
pub enum StripMode {
    /// Strip N items at front, e.g N=1: FOO_BAR_ZOO => BAR_ZOO
    StripFront(u8),
    /// Keeps N items at tail, e.g N=1: FOO_BAR_ZOO => ZOO
    KeepTail(u8),
}

impl StripMode {
    pub fn new(m: i32) -> Self {
        if m < 0 {
            StripMode::KeepTail(m.abs() as u8)
        } else {
            StripMode::StripFront(m as u8)
        }
    }

    pub fn strip_long_name<'a>(&self, name: &'a str) -> &'a str {
        let mut iter = name.match_indices('_');
        let elem = match self {
            StripMode::KeepTail(i) => iter.nth_back((i - 1) as usize),
            StripMode::StripFront(i) => iter.nth((i - 1) as usize),
        };
        let new_name = match elem {
            Some((idx, _)) => &name[idx + 1..name.len()],
            None => {
                eprintln!("{} Not enough length: {}", line!(), name);
                name
            }
        };
        match new_name.chars().take(1).next() {
            None => {
                eprintln!("{} Empty string {}", line!(), name);
                name
            }
            // If after first pass the name starts with a digit (illegal name) do another pass
            Some(c) if c.is_digit(10) => match self {
                StripMode::StripFront(v) => StripMode::StripFront(v + 1),
                StripMode::KeepTail(v) => StripMode::KeepTail(v + 1),
            }
            .strip_long_name(name),
            Some(_) => new_name,
        }
    }
}

static ENUMS: Lazy<HashMap<&str, (&str, i32)>> = Lazy::new(|| {
    // -N translates to StripMode::StripFront(N)
    // N translates to StripMode::KeepTail(N)
    let mut map = HashMap::new();
    map.insert("HAPI_License", ("auto", -2));
    map.insert("HAPI_Result", ("HapiResult", 2));
    map.insert("HAPI_StatusType", ("auto", -2));
    map.insert("HAPI_State", ("auto", 2));
    map.insert("HAPI_PDG_WorkItemState", ("PdgWorkItemState", -1));
    map.insert("HAPI_PDG_EventType", ("PdgEventType", -3));
    map.insert("HAPI_PDG_State", ("PdgState", -1));
    map.insert("HAPI_CacheProperty", ("auto", -2));
    map.insert("HAPI_EnvIntType", ("auto", -2));
    map.insert("HAPI_PrmScriptType", ("auto", -1));
    map.insert("HAPI_Permissions", ("auto", -2));
    map.insert("HAPI_ParmType", ("auto", 2));
    map.insert("HAPI_JobStatus", ("auto", -1));
    map.insert("HAPI_TCP_PortType", ("TcpPortType", -1));
    map.insert("HAPI_ThriftSharedMemoryBufferType", ("auto", -1));
    map.insert("HAPI_PartType", ("auto", -1));
    map.insert("HAPI_StatusVerbosity", ("auto", -1));
    map.insert("HAPI_SessionType", ("auto", -1));
    map.insert("HAPI_PackedPrimInstancingMode", ("auto", -1));
    map.insert("HAPI_RampType", ("auto", -1));
    map.insert("HAPI_ErrorCode", ("auto", -3));
    map.insert("HAPI_NodeFlags", ("auto", -1));
    map.insert("HAPI_NodeType", ("auto", -1));
    map.insert("HAPI_HeightFieldSampling", ("auto", -1));
    map.insert("HAPI_SessionEnvIntType", ("auto", -1));
    map.insert("HAPI_ImagePacking", ("auto", -1));
    map.insert("HAPI_ImageDataFormat", ("auto", -1));
    map.insert("HAPI_XYZOrder", ("auto", -1));
    map.insert("HAPI_RSTOrder", ("auto", -1));
    map.insert("HAPI_TransformComponent", ("auto", -1));
    map.insert("HAPI_CurveOrders", ("auto", -1));
    map.insert("HAPI_InputType", ("auto", -1));
    map.insert("HAPI_GeoType", ("auto", -1));
    map.insert("HAPI_AttributeTypeInfo", ("auto", -1));
    map.insert("HAPI_StorageType", ("auto", 2));
    map.insert("HAPI_VolumeVisualType", ("auto", -1));
    map.insert("HAPI_VolumeType", ("auto", -1));
    map.insert("HAPI_CurveType", ("auto", -1));
    map.insert("HAPI_AttributeOwner", ("auto", -1));
    map.insert("HAPI_GroupType", ("auto", -1));
    map.insert("HAPI_PresetType", ("auto", -1));
    map.insert("HAPI_ChoiceListType", ("auto", -1));
    map.insert("HAPI_InputCurveMethod", ("auto", -1));
    map.insert("HAPI_InputCurveParameterization", ("auto", -1));
    map
});

#[derive(Debug)]
struct Rustifier {
    visited: RefCell<HashMap<String, Vec<String>>>,
}

impl Rustifier {
    fn parse_enum_variant(
        &self,
        _enum_name: Option<&str>,
        _variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Result<Option<String>, String> {
        if _enum_name.is_none() {
            return Ok(None);
        };
        let name = _enum_name
            .ok_or("empty _enum_name".to_string())?
            .strip_prefix("enum ")
            .expect("Not enum?");
        self.visited
            .borrow_mut()
            .entry(name.to_string())
            .and_modify(|variants| variants.push(_variant_name.to_string()))
            .or_default();
        let (_, _mode) = ENUMS
            .get(name)
            .ok_or_else(|| format!("Missing enum: {}", name))?;
        let mode = StripMode::new(*_mode);
        let mut striped = mode.strip_long_name(_variant_name);
        // eprintln!("Paring {name}::{_variant_name} -> {striped}");
        // Two stripped variant names can collide with each other. We take a naive approach by
        // attempting to strip one more time with increased step
        if let Some(vars) = self.visited.borrow_mut().get_mut(name) {
            let _stripped = striped.to_string();
            if vars.contains(&_stripped) {
                println!(
                    "enum {name}::{_variant_name} stripped down to \"{striped}\" is not unique. \
                Incrementing step by 1"
                );
                let mode = StripMode::new(*_mode - 1);
                striped = mode.strip_long_name(_variant_name);
                println!("-> new name is {striped}");
            } else {
                vars.push(_stripped);
            }
        }
        Ok(Some(heck::AsUpperCamelCase(striped).to_string()))
    }
}

impl ParseCallbacks for Rustifier {
    fn enum_variant_name(
        &self,
        _enum_name: Option<&str>,
        _variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<String> {
        match self.parse_enum_variant(_enum_name, _variant_name, _variant_value) {
            Err(e) => {
                eprintln!("Error parsing enum variant: {e}");
                std::process::exit(1);
            }
            Ok(v) => v,
        }
    }

    fn item_name(&self, _item_name: &str) -> Option<String> {
        if let Some((rename, _)) = ENUMS.get(_item_name) {
            let new_name = match *rename {
                "auto" => _item_name
                    .strip_prefix("HAPI_")
                    .expect(&format!("{} - not a HAPI enum?", rename)),
                n => n,
            };
            return Some(new_name.to_string());
        }
        None
    }
}

#[derive(FromArgs, Debug)]
/// Houdini engine raw bindings generator.
struct Args {
    /// absolute path to Houdini install
    #[argh(option)]
    hfs: String,
    /// directory to output the bindings file. Default to CWD.
    #[argh(option)]
    outdir: Option<String>,

    /// rust style naming converntion
    #[argh(option, default = "true")]
    rustify: bool,
}

fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();
    let hfs = Path::new(&args.hfs);
    if !hfs.is_dir() {
        anyhow::bail!("Invalid HFS directory")
    }
    let out_path = match args.outdir.as_ref() {
        Some(dir) => PathBuf::from(dir),
        None => std::env::current_dir()?,
    }
    .join("bindings.rs");

    let include_dir = hfs.join("toolkit/include/HAPI");

    let builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", include_dir.to_string_lossy()))
        .detect_include_paths(true)
        .default_enum_style("rust_non_exhaustive".parse().unwrap())
        .bitfield_enum("NodeType")
        .bitfield_enum("NodeFlags")
        .bitfield_enum("ErrorCode")
        .prepend_enum_name(false)
        .generate_comments(false)
        .derive_copy(true)
        .derive_debug(true)
        .derive_hash(false)
        .derive_eq(false)
        .derive_partialeq(false)
        .disable_name_namespacing()
        // .rustfmt_bindings(true)
        .layout_tests(false)
        .raw_line(format!(
            "// Houdini version {}",
            hfs.file_name().unwrap().to_string_lossy()
        ));
    let builder = if args.rustify {
        let callbacks = Box::new(Rustifier {
            visited: Default::default(),
        });
        builder.parse_callbacks(callbacks)
    } else {
        builder
    };
    builder
        .generate()
        .context("bindgen failed")?
        .write_to_file(out_path.clone())
        .context("Could not write bindings to file")?;
    println!("Generated: {}", out_path.to_string_lossy());
    Ok(())
}
