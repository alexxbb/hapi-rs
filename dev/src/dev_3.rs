pub use hapi_rs::{
    errors::{HapiError, HapiResult, Kind, Result},
    ffi,
    geometry::*,
    node::{HoudiniNode, NodeFlags, NodeType},
    parameter::*,
    session::{CookResult, Session, SessionOptions, StatusVerbosity},
    HOUDINI_VERSION,
};
use hapi_rs::ffi::{PartInfo};
use hapi_rs::StorageType;

pub unsafe fn run() -> Result<()> {
    let mut session = Session::connect_to_pipe("c:/Temp/hars")?;
    // session.cleanup()?;
    let mut opts = SessionOptions::default();
    session.initialize(&opts);
    let otl = std::env::current_dir().unwrap().join("otls/hapi_geo.hda");
    let library = session.load_asset_file(otl.to_string_lossy())?;
    let names = library.get_asset_names()?;
    let node = session.create_node_blocking(&names[0], None, None)?;
    for p in node
        .parameters()?
        .iter()
        .filter(|p| match p.parent().unwrap() {
            None => false,
            Some(p) if p.info().label().unwrap() == "Main" => true,
            _ => false,
        })
    {
        // println!("Name: {}, parent: {:?}", p.name()?, p.parent()?.unwrap().name());
    }

    node.cook_blocking(None)?;
    let geo = node.geometry()?.unwrap();
    let part = geo.part_info(0)?;
    let attribs = geo.get_attribute_names(AttributeOwner::Point, &part)?;
    if geo
        .get_attribute::<f32>(0, AttributeOwner::Prim, "nope")?
        .is_none()
    {
        eprintln!("No attribute: \"nope\"");
    }
    if let Some(attr) = geo.get_attribute::<f32>(0, AttributeOwner::Point, "Cd")? {
        // dbg!(attr.read(0));
    }

    if let Some(attr) = geo.get_attribute::<&str>(0, AttributeOwner::Point, "ptname")? {
        for n in attr.read(0)?.iter_str() {
            println!("{}", n);
        }
    }

    let mut p = PartInfo::new(session.clone());
    dbg!(part.point_count());
    // let info = AttributeInfoBuilder::default()
    //     .count(part.point_count())
    //     .owner(AttributeOwner::Point)
    //     .storage(StorageType::Float)
    //     .build();
    // let attr = geo.add_attribute::<f32>(0, "pscale", &info)?;
    // attr.set(0, &[0.0, 0.1, 0.3])?;

    if let Some(pos) = geo.get_attribute::<f32>(0, AttributeOwner::Point, "P")? {
        for p in pos.read(0)? {
            // println!("{}", p);
        }
    }

    println!("Point groups: {:?}", geo.get_group_names(GroupType::Point)?.iter_str().collect::<Vec<_>>());
    println!("Prim groups: {:?}", geo.get_group_names(GroupType::Prim)?.iter_str().collect::<Vec<_>>());
    Ok(())
}
