use clap::error::ErrorKind;
use clap::Parser;
use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};

const PACKAGE: &str = "hapi-rs";

/// Runs Rust test coverage for the hapi-rs library crate using cargo-llvm-cov.
///
/// With no extra flags, `cargo llvm-cov` runs tests and prints a coverage summary to
/// the terminal. Pass any `cargo llvm-cov` flags after `coverage` (or after `--`).
#[derive(Debug, Parser)]
#[command(
    name = "coverage",
    about = "Run tests and report Rust coverage with cargo-llvm-cov",
    long_about = "Runs `cargo llvm-cov` for the hapi-rs package.\n\n\
        By default (no extra flags), tests run and a human-readable coverage summary is \
        printed to the terminal. Pass any `cargo llvm-cov` option to change the report \
        format or forward arguments to the test binary.",
    after_help = "Examples:
  cargo xtask coverage
  cargo xtask coverage --html
  cargo xtask coverage --json --summary-only
  cargo xtask coverage --open
  cargo xtask coverage test node_
  cargo xtask coverage -- --test-threads 1"
)]
struct Cli {
    /// Arguments forwarded to `cargo llvm-cov` (report format, filters, test args after `--`, etc.).
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        num_args = 0..
    )]
    llvm_cov_args: Vec<String>,
}

pub fn run(workspace_root: &Path, args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut argv = vec!["coverage".to_string()];
    argv.extend(args.iter().cloned());

    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            err.print()?;
            return Ok(());
        }
        Err(err) => return Err(err.into()),
    };

    ensure_cargo_llvm_cov(workspace_root)?;

    let mut command = Command::new("cargo");
    command
        .current_dir(workspace_root)
        .arg("llvm-cov")
        .arg("--package")
        .arg(PACKAGE)
        .args(&cli.llvm_cov_args);

    eprintln!("Running: {}", command_line(&command));
    let status = command.status()?;
    if !status.success() {
        return Err(format!("test coverage failed with {status}").into());
    }

    Ok(())
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
