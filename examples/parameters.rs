use prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR as FORMAT;
use prettytable::*;

use hapi_rs::parameter::{Parameter, ParmType};
use hapi_rs::session::{quick_session, SessionOptions};
use hapi_rs::Result;

fn main() -> Result<()> {
    env_logger::init();
    let opt = SessionOptions::builder().threaded(true).build();
    let session = quick_session(Some(&opt))?;
    let lib = session.load_asset_file("otls/sesi/SideFX_spaceship.hda")?;
    let node = lib.try_create_first()?;
    let asset_parms = lib.get_asset_parms("SideFX::Object/spaceship")?;
    let mut table = prettytable::Table::new();
    table.set_format(*FORMAT);
    table.set_titles(row!["Parameter", "Node Value", "Default Value"]);
    for (node_parm, asset_parm) in node.parameters()?.into_iter().zip(asset_parms.into_iter()) {
        let name = node_parm.label()?;
        let typ = node_parm.info().parm_type();
        let node_val_str = match &node_parm {
            Parameter::Int(p) if typ == ParmType::Toggle => {
                format!("{:?}", if p.get(0)? == 1 { "On" } else { "Off" })
            }
            Parameter::Int(p) => format!("{:?}", p.get(0)?),
            Parameter::Float(p) => {
                if node_parm.size() > 1 {
                    format!("{:?}", p.get_array()?)
                } else {
                    format!("{}", p.get(0)?)
                }
            }
            Parameter::String(p) => {
                let max_len = 30usize;
                let mut val = p.get(0)?;
                val.truncate(max_len);
                if val.len() > 2 {
                    val.push_str("..");
                }
                val
            }
            _ => continue,
        };
        let default_value = format!("{:?}", asset_parm.default_value());
        table.add_row(row![name, node_val_str, default_value]);
    }
    table.printstd();

    Ok(())
}
