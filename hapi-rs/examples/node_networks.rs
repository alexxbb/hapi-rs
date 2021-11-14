use hapi_rs::session::*;
use hapi_rs::node::*;
use hapi_rs::Result;

fn main() -> Result<()> {
    let session = new_in_process()?;
    session.initialize(&SessionOptions::default())?;
    let lib = session.load_asset_file("otls/sesi/FourShapes.hda")?;
    let asset = lib.try_create_first()?;
    let children = asset.get_children(NodeType::Any, NodeFlags::Any, false)?;
    println!("Editable Node Network Child Count: {}", children.len());

    print_child_node(&session, &children)?;

    let box_node = session.create_node_blocking("geo", Some("ProgrammaticBox"), Some(asset.handle))?;

    box_node.connect_input(0, children[0], 0)?;
    // Verify connection
    // let input = box_node.input_node(0)?.expect("Connection");
    // assert_eq!()


    Ok(())
}

fn print_child_node(session: &Session, ids: &Vec<NodeHandle>) -> Result<()> {
    println!("Child Node Ids");
    for handle in ids {
        let info = handle.info(&session)?;
        #[rustfmt::skip]
        println!("\t{} - {}", handle.0, if info.created_post_asset_load() { "NEW" } else { "EXISTING" });
    }

    Ok(())
}