use hapi_rs::{
    session::{Session},
    errors::Result,
};
use smol;

pub unsafe fn run() -> Result<()> {
    let session = Session::new_named_pipe("/tmp/hapi", true)?;
    session.initialize()?;
    let library = session.load_asset_file("/Users/alex/sandbox/rust/hapi/otls/spaceship.otl")?;
    let names = library.get_asset_names();
    println!("{:?}", names.as_ref().unwrap());
    if let Err(e) = names {
        println!("{:?}: {:?}", e.message, e.kind);
    }
    // let node = session.create_node_blocking(&names[0], None, None)?;
    // node.cook_blocking();
    // session.save_hip("/tmp/foo.hip")?;
    Ok(())
}
