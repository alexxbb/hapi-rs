use env_logger::Env;
use hapi_rs::node::*;
use hapi_rs::session::*;
use hapi_rs::Result;

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    let mut session = new_in_process()?;
    session.initialize(&SessionOptions::default())?;

    let lib = session.load_asset_file("otls/sesi/SideFX_spaceship.otl")?;
    let node = lib.try_create_first()?;
    node.cook_blocking(None)?;
    let _asset_info = node.asset_info()?;

    for info in node.get_objects_info()? {
        let obj_node = info.to_node()?;
        if let Some(geometry) = obj_node.geometry()? {
            for part_info in geometry.partitions()? {
                println!(
                    "Object: {}, Display: {}, Partition: {}",
                    obj_node.path(None)?,
                    geometry.node.path(Some(obj_node.handle))?,
                    part_info.part_id()
                );
            }
        }
    }

    Ok(())
}
