use hapi_rs::Result;
use hapi_rs::node::*;
use hapi_rs::session::*;

fn main() -> Result<()> {
    let session = new_thrift_session(SessionOptions::default(), ServerOptions::default())?;
    let lib = session.load_asset_file("../otls/sesi/FourShapes.hda")?;
    let asset = lib.try_create_first()?;
    let children = asset.find_children_by_type(NodeType::Any, NodeFlags::Any, false)?;
    println!("Editable Node Network Child Count: {}", children.len());

    // Print original children
    print_child_node(&session, &children)?;

    // Create a new node and connect one of the child to it
    // let box_node = session.create_node_with("geo", Some("ProgrammaticBox"), &asset, false)?;
    let box_node = session
        .node_builder("geo")
        .with_label("ProgrammaticBox")
        .with_parent(&asset)
        .create()?;
    box_node.connect_input(0, children[0], 0)?;
    // Verify connection
    box_node.input_node(0)?.expect("Connection");

    println!("After CONNECT NODE");
    // Print out children again
    let children = asset.find_children_by_type(NodeType::Any, NodeFlags::Any, false)?;
    print_child_node(&session, &children)?;

    // Delete the new node and print one last time
    box_node.delete()?;
    println!("After DELETING NODE");
    let children = asset.find_children_by_type(NodeType::Any, NodeFlags::Any, false)?;
    print_child_node(&session, &children)?;

    Ok(())
}

fn print_child_node(session: &Session, ids: &[NodeHandle]) -> Result<()> {
    println!("Child Node Ids");
    for handle in ids {
        let info = handle.info(session)?;
        #[rustfmt::skip]
        println!("\t{:?} - {}", handle, if info.created_post_asset_load() {"NEW"} else {"EXISTING"});
    }

    Ok(())
}
