use env_logger;
use hapi_rs::enums::{PdgEventType, PdgWorkItemState};
use hapi_rs::node::{NodeType, Parameter};
use hapi_rs::parameter::ParmBaseTrait;
use hapi_rs::pdg::TopNode;
use hapi_rs::session::{new_in_process, SessionOptionsBuilder};
use hapi_rs::Result;
use std::ops::ControlFlow;

fn cook_async(node: &TopNode) -> Result<Vec<String>> {
    node.dirty_node(true)?;
    let mut all_results = vec![];
    let mut num_tasks = 0;
    let mut tasks_done = 0;
    // TODO: Add progress bar
    println!("Cooking PDG...Should take a couple of seconds");
    node.cook_async(|step| {
        match step.event.event_type() {
            PdgEventType::EventWorkitemAdd => num_tasks += 1,
            PdgEventType::EventWorkitemRemove => num_tasks -= 1,
            PdgEventType::WorkitemStateChange => match step.event.current_state() {
                PdgWorkItemState::Success | PdgWorkItemState::Cache => {
                    let workitem = node.get_workitem(step.event.workitem_id(), step.graph_id)?;
                    if let Some(results) = workitem.get_results()? {
                        all_results.extend(results.into_iter().filter_map(|wir|
                            // We only interested in rendered images
                            (wir.tag().unwrap() == "file/image").then(||wir.result().unwrap())));
                    }
                    tasks_done += 1;
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
    env_logger::init();
    let out_dir = std::env::args().skip(1).next();
    let out_dir = if let Some(out_dir) = out_dir {
        let out_dir = std::path::PathBuf::from(out_dir);
        if !out_dir.exists() {
            eprintln!("Path doesn't exists");
            std::process::exit(1);
        }
        out_dir
    } else {
        eprintln!("Pass a directory path to write PDG output to");
        std::process::exit(1);
    };

    const NODE_TO_COOK: &str = "out";
    const SUBNET: &str = "second";
    const NUM_FRAMES: i32 = 10;
    let otl = std::env::current_dir()
        .unwrap()
        .join("otls/pdg_examples.hda");

    let options = SessionOptionsBuilder::default()
        .threaded(true)
        .env_variables([("JOB", out_dir.to_string_lossy())])
        .build();
    let session = new_in_process(Some(&options))?;
    let lib = session.load_asset_file(&otl)?;
    let asset = lib.try_create_first()?;
    if let Parameter::Float(p) = asset.parameter("num_frames")? {
        p.set(NUM_FRAMES as f32)?;
    }
    if let Parameter::String(p) = asset.parameter("pdg_workingdir").expect("parm") {
        p.set(out_dir.to_string_lossy())?;
    }

    asset.cook_blocking(None)?;

    let subnet = asset.find_child_by_path(SUBNET)?;
    let top_net = &subnet.find_top_networks()?[0];
    let top_node = top_net
        .find_child_by_name(NODE_TO_COOK, NodeType::Top, false)?
        .expect("TOP node");
    let top_node = top_node.to_top_node().expect("top node");
    for output in cook_async(&top_node)? {
        println!("{}", output);
    }
    Ok(())
}
