use anyhow::{anyhow, Result};
use getopts;
mod bindgen;

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
    let header = opts.opt_str("wrapper").unwrap_or("wrapper.h".to_string());
    if ! std::path::Path::new(&header).exists() {
        eprintln!("Can't find wrapper.h");
        std::process::exit(1);
    }
    bindgen::run_bindgen(&opts.opt_str("include").unwrap(),
    &opts.opt_str("wrapper").unwrap(),
    &opts.opt_str("outdir").unwrap())?;
    Ok(())
}
