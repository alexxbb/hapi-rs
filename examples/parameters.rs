use hapi_rs::parameter::{Parameter, ParmBaseTrait};
use hapi_rs::session::{quick_session, SessionOptions};
use hapi_rs::Result;
use prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR as FORMAT;
use prettytable::*;

fn main() -> Result<()> {
    let opt = SessionOptions::builder().threaded(true).build();
    let session = quick_session(Some(&opt))?;
    let lib = session.load_asset_file("otls/sesi/SideFX_spaceship.otl")?;
    let node = lib.try_create_first()?;
    node.cook_blocking(None)?;

    let mut table = prettytable::Table::new();
    table.set_format(*FORMAT);
    table.set_titles(row!["Parameter", "Value"]);
    for parm in node.parameters()? {
        let name = parm.name()?;
        let val_str = match parm {
            Parameter::Int(p) => format!("{:?}", p.get_value()?),
            Parameter::Float(p) => format!("{:?}", p.get_value()?),
            Parameter::String(p) => format!("{:?}", p.get_value()?[0]),
            _ => continue,
        };
        table.add_row(row![name, val_str]);
    }
    table.printstd();

    Ok(())
}
