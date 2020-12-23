use anyhow::{anyhow, Result};
use getopts;
use std::rc::Rc;
use std::path::Path;


mod bindgen;
mod config;
mod helpers;

fn print_help(opts: &getopts::Options) {
    println!("{}", opts.usage("hapi-gen"))
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let mut opts = getopts::Options::new();
    opts.reqopt("", "include", "Include path", "Hfs");
    opts.optopt("", "outdir", "Output directory", "Out");
    opts.optopt("", "config", "codegen.toml", "Config");
    opts.optopt("", "wrapper", "wrapper.h", "Wrapper");
    let opts = match opts.parse(std::env::args()) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{}", e);
            print_help(&opts);
            std::process::exit(1);
        }
    };
    let include = opts
        .opt_str("include")
        .ok_or_else(|| anyhow!("Must provide include"))?;
    let wrapper = opts
        .opt_str("wrapper")
        .ok_or_else(|| anyhow!("Must provide wrapper"))?;
    let outdir = opts
        .opt_str("outdir")
        .ok_or_else(|| anyhow!("Must provide outdir"))?;
    let conf = opts
        .opt_str("config")
        .ok_or_else(|| anyhow!("Must provide codegen.toml"))?;
    let cc = Rc::new(config::read_config(&conf));
    if !std::path::Path::new(&outdir).exists() {
        return Err(anyhow!("Output directory {} doesn't exist", &outdir));
    }

    let output = bindgen::run_bindgen(&include, &wrapper, Rc::clone(&cc))?;
    let bindings_file = Path::new(&outdir).join("bindings.rs");
    std::fs::write(&bindings_file, output.as_bytes()).expect("Writing bindings.rs failed");
    helpers::rustfmt(bindings_file.as_path()).expect("rustfmt failed");

    Ok(())
}
