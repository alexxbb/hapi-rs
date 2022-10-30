use env_logger;
use hapi_rs::enums::{PdgEventType, PdgWorkItemState};
use hapi_rs::node::{NodeType, Parameter};
use hapi_rs::parameter::ParmBaseTrait;
use hapi_rs::pdg::TopNode;
use hapi_rs::session::{new_in_process, quick_session, SessionOptionsBuilder};
use hapi_rs::Result;
use linya::{Bar, Progress};
use std::ops::ControlFlow;

fn cook_async(node: &TopNode) -> Result<Vec<String>> {
    node.dirty_node(true)?;
    let mut all_results = vec![];
    let mut num_tasks = 0;
    let mut tasks_done = 0;
    let mut progress = Progress::new();
    node.cook_async(|step| {
        match step.event.event_type() {
            PdgEventType::EventWorkitemAdd => num_tasks += 1,
            PdgEventType::EventWorkitemRemove => num_tasks -= 1,
            PdgEventType::WorkitemStateChange => match step.event.current_state() {
                PdgWorkItemState::Success | PdgWorkItemState::Cache => {
                    let workitem = node.get_workitem(step.event.workitem_id(), step.graph_id)?;
                    if let Some(results) = workitem.get_results()? {
                        all_results.extend(results.into_iter().map(|wir| wir.result().unwrap()));
                    }
                    tasks_done += 1;
                    let bar = progress.bar(num_tasks, format!("Cooking item {}", workitem.id));
                    progress.set_and_draw(&bar, tasks_done);
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
    let out_dir = std::env::args().skip(1).next();
    let out_dir = if let Some(out_dir) = out_dir {
        let out_dir = std::path::PathBuf::from(out_dir);
        if !out_dir.exists() {
            eprintln!("Path doesn't exists");
            std::process::exit(1);
        }
        out_dir
    } else {
        eprintln!("Pass a directory path to write PDG output");
        std::process::exit(1);
    };
    std::env::set_var("RUST_LOG", "hapi_rs=debug");
    env_logger::init();

    const NODE_TO_COOK: &str = "rbd_sim";
    const SUBNET: &str = "second";
    const NUM_FRAMES: i32 = 10;
    let otl = std::env::current_dir()
        .unwrap()
        .join("otls/pdg_examples.hda");

    let options = SessionOptionsBuilder::default()
        .threaded(false)
        // .env_variables([("JOB", out_dir.to_string_lossy())])
        .build();
    let session = quick_session(Some(&options))?;
    // let session = new_in_process(Some(&options))?;
    session.set_server_var::<str>("JOB", &out_dir.to_string_lossy())?;
    session.set_server_var::<str>("FOO", &out_dir.to_string_lossy())?;
    let lib = session.load_asset_file(&otl)?;
    let asset = lib.try_create_first()?;
    if let Parameter::Float(p) = asset.parameter("num_frames")? {
        p.set_value(&[NUM_FRAMES as f32])?;
    }

    // if let Parameter::String(p) = asset.parameter("pdg_workingdir").expect("parm") {
    //     p.set_value([out_dir.to_string_lossy().to_string()])?;
    // }
    if let Parameter::Button(p) = asset.parameter("push")? {
        p.press_button()?;
    }

    if let Parameter::String(p) = asset.parameter("var").expect("parm") {
        dbg!(p.get_value()?);
    }

    if let Some(Parameter::String(p)) =
        session.find_parameter_from_path("/obj/pdg_examples1/second/colored_sphere/file1/file")?
    {
        dbg!(p.get_value()?);
    }

    return Ok(());

    asset.cook_blocking(None)?;
    let subnet = asset.find_child_by_path(SUBNET)?;
    let top_net = &subnet.find_top_networks()?[0];
    let top_node = top_net
        .find_child_by_name(NODE_TO_COOK, NodeType::Top, false)?
        .expect("TOP node");
    let top_node = top_node.to_top_node().expect("top node");
    for output in cook_async(&top_node)? {
        // println!("{}", output);
    }
    Ok(())
}
