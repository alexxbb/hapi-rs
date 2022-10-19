use std::error::Error;
use hapi_rs::enums::{
    PdgWorkItemState, PdgEventType
};
use hapi_rs::node::{HoudiniNode, NodeFlags, NodeType, Parameter, Session};
use hapi_rs::session::{connect_to_pipe, SessionOptions, SessionOptionsBuilder};
use std::ops::ControlFlow;
use std::path::Path;
use tempfile::TempDir;
use hapi_rs::parameter::ParmBaseTrait;
use hapi_rs::pdg::TopNode;

type  Result<T> = std::result::Result<T, Box<dyn Error>>;

fn cook_sync(node: &TopNode) -> Result<()> {
    todo!()
}

fn cook_async(node: &TopNode) -> Result<()> {
    node
        .cook(|step| {
            match step.event.event_type() {
                PdgEventType::WorkitemStateChange => {
                    match step.event.current_state() {
                        PdgWorkItemState::Success | PdgWorkItemState::Cache => {
                            let workitem = node.get_workitem(step.event.workitem_id(), step.graph_id)?;
                            let results = workitem.get_results()?;
                            for r in &results {
                                let file = r.result(&node.node.session)?;
                                let tag = r.tag(&node.node.session)?;
                                eprintln!("Tag: {}, File: {}", &tag, &file);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            Ok(ControlFlow::Continue(()))
        })?;
    println!("Done");

    Ok(())
}


fn main() -> Result<()> {
    let pipe = Path::new(&std::env::var("TMP").unwrap()).join("hars.pipe");
    let options = SessionOptionsBuilder::default().threaded(true).build();
    let session = connect_to_pipe(pipe, Some(&options))?;
    let otl = std::env::current_dir()
        .unwrap()
        .join("otls/pdg_examples.hda");
    let lib = session.load_asset_file(&otl)?;
    let node = lib.try_create_first()?;

    let tmpdir = tempfile::tempdir()?;

    if let Parameter::String(p) = node.parameter("pdg_workingdir").expect("parm") {
        p.set_value([tmpdir.path().to_string_lossy().to_string()])?;
    }
    let top_net = &node.find_top_networks()?[0];
    let out_node = top_net.find_child_by_name("render_serial", NodeType::Top, false)?.expect("TOP node");
    let out_top = out_node.to_top_node().expect("top node");
    cook_async(&out_top)?;
    Ok(())
}
