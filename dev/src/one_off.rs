/// Example demonstrates how to run a Python script in HARS via an OTL parm callback
use hapi_rs::parameter::{Parameter, ParmBaseTrait};
use hapi_rs::session::{start_engine_pipe_server, Session, SessionOptions};
use hapi_rs::Result;

const SCRIPT: &str = "/Users/alex/CLionProjects/hapi/client/one_off.py";

fn real_main() -> Result<()> {
    start_engine_pipe_server("/tmp/srv", true, 2000.0)?;

    let mut ses = Session::connect_to_pipe("/tmp/srv")?;
    let mut opt = SessionOptions::default();
    opt.unsync = false;
    ses.initialize(&opt);

    ses.load_asset_file("otls/run_script.hda")?;

    let node = ses.create_node_blocking("Object/run_script", None, None)?;
    if let Ok(Parameter::String(p)) = node.parameter("script") {
        p.set_value([SCRIPT.to_string()])?
    }
    if let Ok(Parameter::Int(p)) = node.parameter("run") {
        p.set_value([1])?
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
