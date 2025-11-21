#![allow(unused)]
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use hapi_rs::Result;
use hapi_rs::enums::{PdgEventType, PdgWorkItemState};
use hapi_rs::node::Parameter;
use hapi_rs::pdg::TopNode;
use hapi_rs::session::{SessionOptionsBuilder, new_in_process_session};

fn cook_async(node: &TopNode) -> Result<()> {
    node.dirty_node(true)?;
    let mut num_tasks = 0;
    let mut tasks_done = 0;
    println!("Cooking PDG...");
    node.cook_async(true, None, |step| {
        match step.event.event_type() {
            PdgEventType::EventWorkitemAdd => num_tasks += 1,
            PdgEventType::EventWorkitemRemove => num_tasks -= 1,
            PdgEventType::WorkitemStateChange => match step.event.current_state() {
                PdgWorkItemState::Success | PdgWorkItemState::Cache => {
                    let workitem = node.get_workitem(step.event.workitem_id())?;
                    let results = workitem.get_results()?;
                    for wir in results {
                        // We only interested in geo files;;
                        if wir.tag().unwrap() == "file/geo" {
                            let file = wir.path().unwrap();
                            println!("Completed {file}");
                        }
                        tasks_done += 1;
                    }
                }
                PdgWorkItemState::Fail => {
                    let item = node.get_workitem(step.event.workitem_id())?;
                    eprintln!("Workitem {item:?} failed to cook");
                }
                _ => {}
            },
            PdgEventType::EventCookError => {
                eprintln!("PDG Cook Error :(");
                let msg = step.event.message(&node.node.session)?;
                if !msg.is_empty() {
                    println!("Error: {msg}");
                }
                return Ok(ControlFlow::Break(false));
            }
            _ => {}
        }
        Ok(ControlFlow::Continue(()))
    })?;
    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();

    let hfs: PathBuf = std::env::var_os("HFS").expect("HFS Variable").into();
    let otl = hfs.join("houdini/help/examples/nodes/top/hdaprocessor/example_top_hdaprocessor.hda");
    let out_dir = std::env::args().nth(1);
    let Some(out_dir) = out_dir else {
        eprintln!("Pass a directory path to write PDG output to");
        std::process::exit(1);
    };
    // Paths ending in / cause Houdini to freak out.
    let out_dir: PathBuf = out_dir.trim_end_matches(std::path::MAIN_SEPARATOR).into();
    if !out_dir.exists() {
        eprintln!("Path doesn't exists");
        std::process::exit(1);
    }
    // The example hda uses the HIP variable in TOPs so we have to 'cd'
    std::env::set_current_dir(&out_dir).unwrap();
    let out_dir_str = out_dir.to_string_lossy().to_string();
    unsafe {
        std::env::set_var("PDG_DIR", &out_dir_str);
        std::env::set_var("JOB", &out_dir_str);
    }
    let options = SessionOptionsBuilder::default().threaded(true).build();
    let session = new_in_process_session(Some(options))?;
    let lib = session.load_asset_file(otl)?;
    let asset = lib.try_create_first()?;
    asset.cook_blocking()?;
    session.save_hip(out_dir.join("cook_pdg_example.hip"), true)?;
    let top_net = asset.find_top_networks()?[0].clone();
    let top_node = top_net.to_top_node().expect("TOP node");
    cook_async(&top_node)?;
    Ok(())
}
