use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};

const PACKAGE: &str = "hapi-rs";
const COVERAGE_OUTPUT_DIR: &str = "target/llvm-cov";
const HTML_REPORT_PATH: &str = "target/llvm-cov/html/index.html";

pub fn run(workspace_root: &Path, args: &[String]) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    if options.help {
        print_usage();
        return Ok(());
    }

    ensure_cargo_llvm_cov(workspace_root)?;

    let mut command = Command::new("cargo");
    command
        .current_dir(workspace_root)
        .arg("llvm-cov")
        .arg("--package")
        .arg(PACKAGE);

    if options.html {
        command
            .arg("--html")
            .arg("--output-dir")
            .arg(COVERAGE_OUTPUT_DIR);
    } else {
        command.arg("--summary-only");
    }

    command.args(options.forwarded_args);
    if let Some(test_pattern) = options.test_pattern {
        command.arg("--").arg(test_pattern);
    }

    eprintln!("Running: {}", command_line(&command));
    let status = command.status()?;
    if !status.success() {
        return Err(format!("test coverage failed with {status}").into());
    }

    if options.html {
        println!("HTML coverage report generated at {HTML_REPORT_PATH}");
    }

    Ok(())
}

#[derive(Debug)]
struct Options {
    html: bool,
    help: bool,
    test_pattern: Option<String>,
    forwarded_args: Vec<String>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, Box<dyn Error>> {
        let mut html = false;
        let mut help = false;
        let mut test_pattern = None;
        let mut forwarded_args = Vec::new();
        let mut forward_rest = false;
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            if forward_rest {
                forwarded_args.push(arg.clone());
                continue;
            }

            match arg.as_str() {
                "--html" => html = true,
                "-h" | "--help" => help = true,
                "--" => forward_rest = true,
                "--pattern" => {
                    let Some(pattern) = iter.next() else {
                        return Err("--pattern requires a value".into());
                    };
                    test_pattern = Some(pattern.clone());
                }
                arg if arg.starts_with("--pattern=") => {
                    let pattern = arg
                        .split_once('=')
                        .map(|(_, pattern)| pattern)
                        .filter(|pattern| !pattern.is_empty())
                        .ok_or("--pattern requires a value")?;
                    test_pattern = Some(pattern.to_string());
                }
                _ => forwarded_args.push(arg.clone()),
            }
        }

        Ok(Self {
            html,
            help,
            test_pattern,
            forwarded_args,
        })
    }
}

fn ensure_cargo_llvm_cov(workspace_root: &Path) -> Result<(), Box<dyn Error>> {
    let status = Command::new("cargo")
        .current_dir(workspace_root)
        .args(["llvm-cov", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if status.success() {
        return Ok(());
    }

    Err(
        "cargo-llvm-cov is required. Install it with `cargo install cargo-llvm-cov` and rerun this command."
            .into(),
    )
}

fn command_line(command: &Command) -> String {
    let mut parts = Vec::new();
    parts.push(command.get_program().to_string_lossy().into_owned());
    parts.extend(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned()),
    );
    parts.join(" ")
}

fn print_usage() {
    println!(
        "Usage: cargo xtask test-coverage [--html] [--pattern <pattern>] [-- <cargo-llvm-cov args>]

Runs Rust test coverage for the hapi-rs library crate using cargo-llvm-cov.

Options:
  --html                    Generate an HTML report at target/llvm-cov/html/index.html
  --pattern <pattern>       Only run tests whose names contain the pattern

Examples:
  cargo xtask test-coverage
  cargo xtask test-coverage --html
  cargo xtask test-coverage --pattern node_"
    );
}
