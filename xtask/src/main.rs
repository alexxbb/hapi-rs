use std::env;
use std::path::PathBuf;
use std::process;

mod ffi_coverage;
mod hapi_bindgen;
mod test_coverage;

fn main() {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage_and_exit(0);
    };
    let mut extra_args: Vec<String> = args.collect();
    strip_optional_separator(&mut extra_args);

    let workspace_root = workspace_root();
    let exit_code = match command.as_str() {
        "bindgen" => run_command(hapi_bindgen::run(&workspace_root, &extra_args)),
        "ffi-coverage" => run_command(ffi_coverage::run(&workspace_root, &extra_args)),
        "test-coverage" => run_command(test_coverage::run(&workspace_root, &extra_args)),
        "-h" | "--help" | "help" => {
            print_usage_and_exit(0);
        }
        _ => {
            eprintln!("Unknown xtask command: {command}");
            print_usage_and_exit(2);
        }
    };

    process::exit(exit_code);
}

fn run_command(result: Result<(), impl std::fmt::Display>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{err}");
            2
        }
    }
}

fn strip_optional_separator(args: &mut Vec<String>) {
    if matches!(args.first(), Some(first) if first == "--") {
        args.remove(0);
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate must be in workspace root/xtask")
        .to_path_buf()
}

fn print_usage_and_exit(code: i32) -> ! {
    println!(
        "Usage: cargo xtask <command> [-- <args>]

Commands:
  bindgen        Generate lib/src/ffi/bindings.rs
  ffi-coverage   Report wrapped vs raw HAPI coverage
  test-coverage  Report Rust test coverage with cargo-llvm-cov

Examples:
  cargo xtask bindgen
  cargo xtask bindgen -- --outdir /tmp
  cargo xtask ffi-coverage
  cargo xtask test-coverage
  cargo xtask test-coverage --html
  cargo xtask test-coverage --json"
    );
    process::exit(code);
}
