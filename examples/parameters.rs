use hapi_rs::parameter::{Parameter, ParmType};
use hapi_rs::session::{quick_session, SessionOptions};
use hapi_rs::Result;
use prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR as FORMAT;
use prettytable::*;

fn main() -> Result<()> {
    env_logger::init();
    let opt = SessionOptions::builder().threaded(true).build();
    let session = quick_session(Some(&opt))?;
    let lib = session.load_asset_file("otls/sesi/SideFX_spaceship.hda")?;
    let node = lib.try_create_first()?;
    node.cook_blocking()?;

    let mut table = prettytable::Table::new();
    table.set_format(*FORMAT);
    table.set_titles(row!["Parameter", "Value"]);
    for parm in node.parameters()? {
        let name = parm.name()?;
        let typ = parm.info().parm_type();
        let val_str = match &parm {
            Parameter::Int(p) if typ == ParmType::Toggle => {
                format!("{:?}", if p.get(0)? == 1 { "On" } else { "Off" })
            }
            Parameter::Int(p) => format!("{:?}", p.get(0)?),
            Parameter::Float(p) => format!("{:?}", p.get(0)?),
            Parameter::String(p) => format!("{:?}", p.get(0)?),
            _ => continue,
        };
        table.add_row(row![name, val_str]);
    }
    table.printstd();

    Ok(())
}
