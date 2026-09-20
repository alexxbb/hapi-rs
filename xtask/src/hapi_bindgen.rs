use anyhow::Context;
use bindgen::callbacks::{EnumVariantValue, ParseCallbacks};
use once_cell::sync::Lazy;
use regex_lite::Regex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

const HEADER_PATH: &str = "xtask/src/wrapper.h";
const DEFAULT_OUTDIR: &str = "lib/src/ffi";

#[derive(Debug, Copy, Clone)]
enum StripMode {
    StripFront(u8),
    KeepTail(u8),
}

impl StripMode {
    fn new(mode: i32) -> Self {
        if mode < 0 {
            Self::KeepTail(mode.abs() as u8)
        } else {
            Self::StripFront(mode as u8)
        }
    }

    fn strip_long_name<'a>(&self, name: &'a str) -> &'a str {
        let mut iter = name.match_indices('_');
        let elem = match self {
            Self::KeepTail(i) => iter.nth_back((i - 1) as usize),
            Self::StripFront(i) => iter.nth((i - 1) as usize),
        };
        let new_name = match elem {
            Some((idx, _)) => &name[idx + 1..],
            None => {
                eprintln!("Not enough length: {name}");
                name
            }
        };
        match new_name.chars().next() {
            None => {
                eprintln!("Empty string {name}");
                name
            }
            Some(c) if c.is_ascii_digit() => match self {
                Self::StripFront(v) => Self::StripFront(v + 1),
                Self::KeepTail(v) => Self::KeepTail(v + 1),
            }
            .strip_long_name(name),
            Some(_) => new_name,
        }
    }
}

static ENUMS: Lazy<HashMap<&str, (&str, i32)>> = Lazy::new(|| {
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
    map.insert("HAPI_CameraProjectionType", ("auto", -1));
    map
});

#[derive(Debug)]
struct Rustifier {
    visited: RefCell<HashMap<String, Vec<String>>>,
}

impl Rustifier {
    fn parse_enum_variant(
        &self,
        enum_name: Option<&str>,
        variant_name: &str,
        variant_value: EnumVariantValue,
    ) -> Result<Option<String>, String> {
        let _ = variant_value;
        if enum_name.is_none() {
            return Ok(None);
        }
        let name = enum_name
            .ok_or_else(|| "empty enum_name".to_string())?
            .strip_prefix("enum ")
            .expect("Not enum?");
        self.visited
            .borrow_mut()
            .entry(name.to_string())
            .and_modify(|variants| variants.push(variant_name.to_string()))
            .or_default();
        let (_, mode_value) = ENUMS
            .get(name)
            .ok_or_else(|| format!("Missing enum: {name}"))?;
        let mode = StripMode::new(*mode_value);
        let mut stripped = mode.strip_long_name(variant_name);
        if let Some(vars) = self.visited.borrow_mut().get_mut(name) {
            let new_name = stripped.to_string();
            if vars.contains(&new_name) {
                println!(
                    "enum {name}::{variant_name} stripped down to \"{stripped}\" is not unique. \
Incrementing step by 1"
                );
                let mode = StripMode::new(*mode_value - 1);
                stripped = mode.strip_long_name(variant_name);
                println!("-> new name is {stripped}");
            } else {
                vars.push(new_name);
            }
        }
        Ok(Some(heck::AsUpperCamelCase(stripped).to_string()))
    }
}

impl ParseCallbacks for Rustifier {
    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        variant_name: &str,
        variant_value: EnumVariantValue,
    ) -> Option<String> {
        match self.parse_enum_variant(enum_name, variant_name, variant_value) {
            Err(err) => {
                eprintln!("Error parsing enum variant: {err}");
                std::process::exit(1);
            }
            Ok(value) => value,
        }
    }

    fn item_name(&self, item_name: &str) -> Option<String> {
        if let Some((rename, _)) = ENUMS.get(item_name) {
            let new_name = match *rename {
                "auto" => item_name
                    .strip_prefix("HAPI_")
                    .expect("Not a HAPI enum name"),
                other => other,
            };
            return Some(new_name.to_string());
        }
        None
    }
}

pub fn run(workspace_root: &Path, args: &[String]) -> anyhow::Result<()> {
    let parsed = parse_args(args, workspace_root)?;

    let hfs_env =
        env::var("HFS").expect("HFS environment variable must be set for `cargo xtask bindgen`");
    let hfs = Path::new(&hfs_env);
    if !hfs.is_dir() {
        anyhow::bail!("Invalid HFS directory");
    }
    let houdini_version = read_houdini_version(hfs)?;

    let out_dir = parsed.outdir;
    let out_path = out_dir.join("bindings.rs");
    let include_dir = hfs.join("toolkit/include/HAPI");
    let header_path = workspace_root.join(HEADER_PATH);

    let builder = bindgen::Builder::default()
        .header(header_path.to_string_lossy())
        .clang_arg(format!("-I{}", include_dir.to_string_lossy()))
        .detect_include_paths(true)
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: true,
        })
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
        .layout_tests(false)
        .raw_line(format!("// Houdini version {}", houdini_version));

    let builder = builder.parse_callbacks(Box::new(Rustifier {
        visited: Default::default(),
    }));

    builder
        .generate()
        .context("bindgen failed")?
        .write_to_file(&out_path)
        .context("Could not write bindings to file")?;

    println!("Generated: {}", out_path.to_string_lossy());
    Ok(())
}

fn read_houdini_version(hfs: &Path) -> anyhow::Result<String> {
    let version_path = hfs.join("toolkit/cmake/HoudiniConfigVersion.cmake");
    let raw = std::fs::read_to_string(&version_path).with_context(|| {
        format!(
            "Could not read Houdini config version file at {}",
            version_path.to_string_lossy()
        )
    })?;
    let version_regex =
        Regex::new(r#"set\(\s*PACKAGE_VERSION\s+([0-9]+\.[0-9]+\.[0-9]+)\s*\)"#).unwrap();
    let captures = version_regex.captures(&raw).ok_or_else(|| {
        anyhow::anyhow!(
            "Could not find `set( PACKAGE_VERSION X.Y.Z )` in {}",
            version_path.to_string_lossy()
        )
    })?;
    Ok(captures[1].to_string())
}

#[derive(Debug)]
struct ParsedArgs {
    outdir: PathBuf,
}

fn parse_args(args: &[String], workspace_root: &Path) -> anyhow::Result<ParsedArgs> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        std::process::exit(0);
    }

    let mut outdir: Option<PathBuf> = None;

    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--outdir" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("Missing value for --outdir"))?;
                outdir = Some(PathBuf::from(value));
            }
            value if let Some(v) = value.strip_prefix("--outdir=") => {
                outdir = Some(PathBuf::from(v));
            }
            other => {
                return Err(anyhow::anyhow!("Unrecognized argument: {other}"));
            }
        }
        i += 1;
    }

    Ok(ParsedArgs {
        outdir: outdir.unwrap_or_else(|| workspace_root.join(DEFAULT_OUTDIR)),
    })
}

fn print_help() {
    println!(
        "Usage: cargo xtask bindgen [-- <options>]

Options:
  --outdir <path>      output directory (defaults to lib/src/ffi)
  --help               show this help"
    );
}
