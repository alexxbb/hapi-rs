/// connecting_assets.cpp
use hapi_rs::{attribute::*, geometry::*, session::*};
use std::error::Error;

fn main() -> Result<()> {
    let mut session = new_in_process()?;
    session.initialize(&SessionOptions::default())?;
    let new_node = session.create_input_node("Cube")?;
    new_node.cook_blocking(None)?;
    let part_info = PartInfo::default()
        .with_part_type(PartType::Mesh)
        .with_face_count(6)
        .with_vertex_count(24)
        .with_point_count(8);
    let geom = new_node.geometry()?.expect("geometry");
    geom.set_part_info(&part_info);
    let p_info = AttributeInfo::default()
        .with_count(8)
        .with_tuple_size(3)
        .with_storage(StorageType::Float)
        .with_owner(AttributeOwner::Point);
    let p_attrib = geom.add_attribute::<f32>("P", 0, &p_info)?;

    #[rustfmt::skip]
        let positions: [f32; 24] = [
        0.0, 0.0, 0.0,
        0.0, 0.0, 1.0,
        0.0, 1.0, 0.0,
        0.0, 1.0, 1.0,
        1.0, 0.0, 0.0,
        1.0, 0.0, 1.0,
        1.0, 1.0, 0.0,
        1.0, 1.0, 1.0
    ];

    p_attrib.set(0, &positions)?;

    #[rustfmt::skip]
        let vertices: [i32; 24] = [
        0, 2, 6, 4,
        2, 3, 7, 6,
        2, 0, 1, 3,
        1, 5, 7, 3,
        5, 4, 6, 7,
        0, 4, 5, 1
    ];

    geom.set_vertex_list(0, &vertices)?;
    geom.set_face_counts(0, &[4, 4, 4, 4, 4, 4])?;
    geom.commit()?;

    let subdivide_node = session.create_node("Sop/subdivide", Some("Cube Subdivider"), None)?;
    subdivide_node.connect_input(0, new_node, 0)?;
    session.save_hip("connecting_assets.hip", false)?;
    println!("Saving connecting_assets.hip");
    Ok(())
}
