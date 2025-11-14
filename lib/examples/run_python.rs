// Houdini Engine does not allow execution of Python code directly,
// but this example uses an otl which runs a provided snippet or a file
use hapi_rs::Result;
use hapi_rs::parameter::Parameter;
use hapi_rs::session::quick_session;

const SCRIPT: &str = r#"
import hou
hou.hscript('set -g TEST=hapi')
"#;

fn main() -> Result<()> {
    let ses = quick_session(None)?;
    let lib = ses.load_asset_file("../otls/hapi_script.hda")?;
    let node = lib.try_create_first()?;
    if let Parameter::String(parm) = node.parameter("code")? {
        parm.set(0, SCRIPT)?;
    }
    if let Parameter::Button(parm) = node.parameter("run")? {
        parm.press_button()?
    }
    let val = ses.get_server_var::<str>("TEST")?;
    assert_eq!(val, "hapi");
    Ok(())
}
