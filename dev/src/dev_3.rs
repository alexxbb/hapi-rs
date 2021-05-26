pub use hapi_rs::{
    Result,
    PartType,
    ffi,
    geometry::*,
    node::{HoudiniNode, NodeFlags, NodeType},
    parameter::*,
    session::{CookResult, Session, SessionOptions, StatusVerbosity},
    HOUDINI_VERSION,
};
use hapi_rs::ffi::{PartInfo, AttributeInfo};
use hapi_rs::StorageType;

pub unsafe fn run() -> Result<()> {
    let mut session = Session::connect_to_pipe("c:/Temp/hars")?;
    session.cleanup()?;
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
    let err = session.get_cook_result_string(StatusVerbosity::Statusverbosity2)?;
    println!("Status: {}", err);
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

    /*
       newNodePart.type = HAPI_PARTTYPE_MESH;
    newNodePart.faceCount = 1;
    newNodePart.vertexCount = 3;
    newNodePart.pointCount = 3;


        newNodePointInfo.count = 8;
    newNodePointInfo.tupleSize = 3;
    newNodePointInfo.exists = true;
    newNodePointInfo.storage = HAPI_STORAGETYPE_FLOAT;
    newNodePointInfo.owner = HAPI_ATTROWNER_POINT;

    ENSURE_SUCCESS( HAPI_AddAttribute( &session, newNode, 0, "P", &newNodePointInfo ) );

    float positions[ 24 ] = { 0.0f, 0.0f, 0.0f,   // 0
			      0.0f, 0.0f, 1.0f,   // 1
			      0.0f, 1.0f, 0.0f,   // 2
			      0.0f, 1.0f, 1.0f,   // 3
			      1.0f, 0.0f, 0.0f,   // 4
			      1.0f, 0.0f, 1.0f,   // 5
			      1.0f, 1.0f, 0.0f,   // 6
			      1.0f, 1.0f, 1.0f }; // 7
     */

    let node = session.create_input_node("input")?;
    node.cook_blocking(None)?;
    let geo = node.geometry()?.expect("Input geo must exist");
    let part = PartInfo::default()
        .with_part_type(PartType::Mesh)
        .with_face_count(1)
        .with_point_count(3)
        .with_vertex_count(3);
    geo.set_part_info(&part)?;
    let info = AttributeInfo::default()
        .with_count(part.point_count())
        .with_tuple_size(3)
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Float);
    let attr = geo.add_attribute::<f32>(0, "P", &info)?;
    attr.set(part.part_id(), &[
        0.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        1.0, 0.0, 0.0
    ])?;
    geo.set_vertex_list(0, [0,1,2])?;
    geo.set_face_counts(0, [3])?;
    geo.commit()?;
    // let attr = geo.add_attribute::<f32>(0, "pscale", &info)?;


    geo.save_to_file("c:/temp/debug.geo")?;
    session.save_hip("c:/temp/debug.hip")?;

    // if let Some(pos) = geo.get_attribute::<f32>(0, AttributeOwner::Point, "P")? {
    //     for p in pos.read(0)? {
    //         // println!("{}", p);
    //     }
    // }
    //
    // println!("Point groups: {:?}", geo.get_group_names(GroupType::Point)?.iter_str().collect::<Vec<_>>());
    // println!("Prim groups: {:?}", geo.get_group_names(GroupType::Prim)?.iter_str().collect::<Vec<_>>());
    Ok(())
}
