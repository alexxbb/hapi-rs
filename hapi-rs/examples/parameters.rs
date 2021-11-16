use hapi_rs::parameter::{Parameter, ParmBaseTrait};
use hapi_rs::session::{new_in_process, SessionOptions};
use hapi_rs::Result;
use prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR as FORMAT;
use prettytable::*;

fn main() -> Result<()> {
    let mut session = new_in_process()?;
    let mut opt = SessionOptions::default();
    opt.threaded = true;
    session.initialize(&opt)?;
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
