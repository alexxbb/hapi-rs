/// Example demonstrates how to run a Python script in HARS via an OTL parm callback
use hapi_rs::*;
use hapi_rs::session::*;
use hapi_rs::parameter::*;
// use hapi_rs::Result;

#[cfg(windows)]
const SCRIPT: &str = "C:/Users/houal/sandbox/hapi-rs/client/one_off.py";
#[cfg(macos)]
const SCRIPT: &str = "/Users/alex/CLionProjects/hapi/client/one_off.py";

fn real_main() -> Result<()> {
    let ses = simple_session(None)?;
    let lib = ses.load_asset_file("otls/run_script.hda")?;
    let node = lib.try_create_first()?;
    if let Ok(Parameter::String(p)) = node.parameter("script") {
        p.set_value([SCRIPT.to_string()])?
    }
    if let Ok(Parameter::Button(p)) = node.parameter("run") {
        p.press_button()?
    }

    Ok(())
}

fn main() {
    match real_main() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}
