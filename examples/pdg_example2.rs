#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]

use env_logger;
use hapi_rs::enums::{PdgEventType, PdgWorkItemState};
use hapi_rs::node::{HoudiniNode, NodeFlags, NodeType, Parameter, Session};
use hapi_rs::parameter::ParmBaseTrait;
use hapi_rs::pdg::TopNode;
use hapi_rs::session::{
    connect_to_pipe, new_in_process, quick_session, SessionOptions, SessionOptionsBuilder,
};
use hapi_rs::{ErrorContext, Result};
use std::error::Error;
use std::ops::ControlFlow;
use std::path::Path;
use tempfile::TempDir;


fn cook_blocking(node: &TopNode) -> Result<Vec<String>> {
    node.cook_blocking()?.into_iter().map(|wir|dbg!(wir.result())).collect()
}

fn cook_async(node: &TopNode) -> Result<Vec<String>> {
    node.dirty_node(true)?;
    let mut all_results = vec![];
    node.cook_async(|step| {
        match step.event.event_type() {
            PdgEventType::WorkitemStateChange => match step.event.current_state() {
                PdgWorkItemState::Success | PdgWorkItemState::Cache => {
                    let workitem = node.get_workitem(step.event.workitem_id(), step.graph_id)?;
                    if let Some(results) = workitem.get_results()? {
                        all_results.extend(results.into_iter().map(|wir| wir.result().unwrap()));
                    }
                }
                PdgWorkItemState::Fail => {
                    let item = node.get_workitem(step.event.workitem_id(), step.graph_id)?;
                    eprintln!("Workitem {item:?} failed to cook");
                }
                _ => {}
            },
            PdgEventType::EventCookError => {
                eprintln!("PDG Cook Error :(");
                return Ok(ControlFlow::Break(false));
            }
            _ => {}
        }
        Ok(ControlFlow::Continue(()))
    })?;
    Ok(all_results)
}

fn main() -> Result<()> {
    const NODE_TO_COOK: &str = "rbd_sim";

    env_logger::init();
    // let _tmpdir = tempfile::tempdir()?;
    // let tmpdir = _tmpdir.path().to_owned();
    // std::mem::forget(_tmpdir);

    let tmpdir = Path::new(r"C:\Temp\pdg\sim");

    let options = SessionOptionsBuilder::default()
        .threaded(true)
        .env_variables([("JOB", tmpdir.to_string_lossy())])
        .build();
    let session = quick_session(Some(&options))?;
    // let session = new_in_process(Some(&options))?;
    let otl = std::env::current_dir()
        .unwrap()
        .join("otls/pdg_examples.hda");
    let lib = session.load_asset_file(&otl)?;
    let node = lib.try_create_first()?;
    node.cook_blocking(None)?;
    let second = node.find_child_by_path("second")?;

    if let Parameter::String(p) = node.parameter("pdg_workingdir").expect("parm") {
        p.set_value([tmpdir.to_string_lossy().to_string()])?;
    }
    let top_net = &second.find_top_networks()?[0];

    let render = top_net
        .find_child_by_name(NODE_TO_COOK, NodeType::Top, false)?
        .expect("TOP node");
    let render = render.to_top_node().expect("top node");
    let results = cook_blocking(&render)?;
    for r in results {
        println!("{}", r);
    }
    Ok(())
}
