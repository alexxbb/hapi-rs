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
    let _top_nodes = top_net.get_children(NodeType::Top, NodeFlags::Nonscheduler, false)?;
    for n in _top_nodes {
        let top_node = n.as_top_node(&session)?.unwrap();
        let mut num = 0;
        top_node.cook(|info, _| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            dbg!(info.event_type());
            if info.event_type() == PdgEventType::EventWorkitemAdd {
                num += 1;
            }
            ControlFlow::Continue(())
        })?;
        dbg!(num);
    }
    // dbg!(&node.name()?);
    Ok(())
}
