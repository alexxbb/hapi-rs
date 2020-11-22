use hapi_rs::{
    session::{Session},
    errors::Result,
};
use smol;

pub unsafe fn run() -> Result<()> {
    let session = Session::new_named_pipe("/tmp/hapi", true)?;
    session.initialize()?;
    let library = session.load_asset_file("/Users/alex/sandbox/rust/hapi/otls/spaceship.otl")?;
    let names = library.get_asset_names()?;
    let node = session.create_node(&names[0], None, None)?;
    smol::block_on(async {
        node.cook().await
    })?;
    // let cook = node.cook_async();
    // smol::block_on(smol::unblock(move || cook.complete()));
    // println!("{:?}", names);
    Ok(())
}
