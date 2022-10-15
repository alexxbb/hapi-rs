use hapi_rs::enums::PdgEventType;
use hapi_rs::node::{NodeFlags, NodeType};
use hapi_rs::session::connect_to_pipe;
use hapi_rs::Result;
use std::ops::ControlFlow;
use std::path::Path;

fn main() -> Result<()> {
    let pipe = Path::new(&std::env::var("TMP").unwrap()).join("hars.pipe");
    let session = connect_to_pipe(pipe, None)?;
    let otl = std::env::current_dir().unwrap().join("otls/hapi_pdg.hda");
    let lib = session.load_asset_file(otl)?;
    let node = lib.try_create_first()?;
    node.cook_blocking(None)?;
    let networks = node.find_top_networks()?;
    let top_net = &networks[0];
    let node = top_net
        .find_child("out", NodeType::Top, false)?
        .expect("out node");
    let out_top = node.as_top_node().expect("top node");
    out_top.cook(|info, _ctx_name| {
        ControlFlow::Continue(())
    })?;
    Ok(())
}
