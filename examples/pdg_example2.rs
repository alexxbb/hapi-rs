use hapi_rs::enums::PdgEventType;
use hapi_rs::node::{NodeFlags, NodeType};
use hapi_rs::session::connect_to_pipe;
use hapi_rs::Result;
use std::ops::ControlFlow;
use std::path::Path;

fn main() -> Result<()> {
    let pipe = Path::new(&std::env::var("TMP").unwrap()).join("hars.pipe");
    let hip = std::env::current_dir()
        .unwrap()
        .join("otls/pdg_examples.hip");
    let session = connect_to_pipe(pipe, None)?;
    session.load_hip(hip, false)?;
    let networks = session
        .get_manager_node(NodeType::Obj)?
        .find_network_nodes(NodeType::Top)?;
    let top_net = &networks[0];
    // let node = top_net
    //     .find_child("out", NodeType::Top, false)?
    //     .expect("out node");
    // let out_top = node.to_top_node().expect("top node");
    // std::thread::spawn(move || {
    //     out_top
    //         .cook(|info, _ctx_name| {
    //             dbg!(_ctx_name);
    //             ControlFlow::Continue(())
    //             // ControlFlow::Break(true)
    //         })
    //         .unwrap();
    // })
    // .join()
    // .unwrap();
    Ok(())
}
