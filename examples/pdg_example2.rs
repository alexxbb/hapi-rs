#![allow(dead_code)]

use env_logger;
use std::error::Error;
use hapi_rs::enums::{
    PdgWorkItemState, PdgEventType
};
use hapi_rs::node::{HoudiniNode, NodeFlags, NodeType, Parameter, Session};
use hapi_rs::session::{connect_to_pipe, quick_session, SessionOptions, SessionOptionsBuilder};
use std::ops::ControlFlow;
use std::path::Path;
use tempfile::TempDir;
use hapi_rs::parameter::ParmBaseTrait;
use hapi_rs::pdg::TopNode;
use hapi_rs::{ErrorContext, Result};

// type  Result<T> = std::result::Result<T, Box<dyn Error>>;

fn cook_sync(_node: &TopNode) -> Result<()> {
    todo!()
}

fn cook_async(node: &TopNode) -> Result<()> {
    node.dirty_node(true)?;
    node
        .cook(|step| {
            match step.event.event_type() {
                PdgEventType::WorkitemStateChange => {
                    match step.event.current_state() {
                        PdgWorkItemState::Success | PdgWorkItemState::Cache => {
                            let workitem = node.get_workitem(step.event.workitem_id(), step.graph_id)?;
                            let results = workitem.get_results()?;
                            for r in &results {
                                let file = r.result(&node.node.session).context("Getting result")?;
                                let tag = r.tag(&node.node.session).expect("File tag");
                                eprintln!("Tag: {}, File: {}", &tag, &file);
                            }
                        }
                        PdgWorkItemState::Fail => {
                            let info = node.node.session.get_string(step.graph_name)?;
                            // let workitem = node.get_workitem(step.event.workitem_id(), step.graph_id)?;
                            eprintln!("Failed to cook: {}", info);
                        }
                        _ => {}
                    }
                }
                PdgEventType::EventCookError => {
                    eprintln!("PDG Cook Error :(");
                    return Ok(ControlFlow::Break(false));
                }
                _ => {}
            }
            Ok(ControlFlow::Continue(()))
        })?;
    println!("Done");

    Ok(())
}


fn main() -> Result<()> {
    env_logger::init();
    // let _tmpdir = tempfile::tempdir()?;
    // let tmpdir = _tmpdir.path().to_owned();
    // std::mem::forget(_tmpdir);

    let tmpdir = Path::new(r"C:\Temp\pdg\sim");

    let options = SessionOptionsBuilder::default().threaded(true)
        .cleanup_on_close(true)
        .env_variables([("JOB", tmpdir.to_string_lossy())])
        .build();
    let session = quick_session(Some(&options))?;
    let otl = std::env::current_dir()
        .unwrap()
        .join("otls/pdg_examples.hda");
    let lib = session.load_asset_file(&otl)?;
    let node = lib.try_create_first()?;
    // if let Parameter::Button(p) = node.parameter("push").expect("push") {
    //     p.press_button()?;
    // }
    //
    // if let Parameter::String(p) = node.parameter("var").expect("var") {
    //     eprintln!("VAR: {}", p.get_value()?[0]);
    // }

    // return Ok(());

    let second = node.find_child_by_path("second")?;

    dbg!(tmpdir);
    session.save_hip(&tmpdir.join("foo.hip").to_string_lossy(), false)?;

    if let Parameter::String(p) = node.parameter("pdg_workingdir").expect("parm") {
        p.set_value([tmpdir.to_string_lossy().to_string()])?;
    }
    let top_net = &second.find_top_networks()?[0];

    let render = top_net.find_child_by_name("rbd_sim", NodeType::Top, false)?.expect("TOP node");
    let render = render.to_top_node().expect("top node");
    if let Parameter::String(p) = render.node.parameter("sopoutput").unwrap() {
        dbg!(p.get_value().unwrap());
    }
    if let Err(e) = cook_async(&render) {
        eprintln!("PDG Cooked with error: {:?}", e);
    }
    Ok(())
}
