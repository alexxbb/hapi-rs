use hapi_rs::parameter::Parameter;
use hapi_rs::server::ServerOptions;
use hapi_rs::session::{SessionOptions, new_thrift_session};

fn main() -> anyhow::Result<()> {
    let session = new_thrift_session(
        SessionOptions::default(),
        ServerOptions::pipe_with_defaults(),
    )?;

    let topnet = session.create_node("Object/topnet")?;
    let generator = topnet
        .session
        .node_builder("genericgenerator")
        .with_parent(&topnet)
        .create()?
        .to_top_node()
        .expect("TOP node");

    generator.node.cook_blocking()?;
    let workitem = generator.create_workitem("test_1", 0, None)?;
    workitem.set_int_data("my_int_data", &[1, 2, 3])?;
    generator.commit_workitems()?;
    generator.cook_pdg_blocking(false)?;
    let script_node = session
        .node_builder("pythonscript")
        .with_parent(topnet)
        .create()?;
    if let Parameter::String(parm) = script_node.parameter("script")? {
        let code = "\
        data = work_item.data.intDataArray(\"my_int_data\")\n\
        work_item.data.setIntArray(\"my_int_data\", [v**v for v in data])";
        parm.set(0, code)?;
    }
    script_node.connect_input(0, generator, 0)?;
    script_node.set_display_flag(true)?;
    let script_node = script_node.to_top_node().unwrap();
    script_node.cook_pdg_blocking(false)?;
    let workitems = script_node.get_all_workitems()?;
    let data = workitems[0].get_int_data("my_int_data")?;
    assert_eq!(data, vec![1, 4, 27]);

    Ok(())
}
