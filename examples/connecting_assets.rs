// connecting_assets.cpp
use hapi_rs::Result;
use hapi_rs::{attribute::*, geometry::*, session::*};

fn main() -> Result<()> {
    let session = quick_session(None)?;
    let geom = session.create_input_node("Cube")?;
    geom.node.cook_blocking()?;
    let part_info = PartInfo::default()
        .with_part_type(PartType::Mesh)
        .with_face_count(6)
        .with_vertex_count(24)
        .with_point_count(8);
    geom.set_part_info(&part_info)?;
    let p_info = AttributeInfo::default()
        .with_count(8)
        .with_tuple_size(3)
        .with_storage(StorageType::Float)
        .with_owner(AttributeOwner::Point);
    let p_attrib = geom.add_numeric_attribute("P", 0, p_info)?;

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

    geom.set_vertex_list(0, vertices)?;
    geom.set_face_counts(0, [4, 4, 4, 4, 4, 4])?;
    geom.commit()?;

    let subdivide_node = session.create_node("Sop/subdivide", Some("Cube Subdivide"), None)?;
    subdivide_node.connect_input(0, geom.node, 0)?;
    let hip = std::env::temp_dir().join("connecting_assets.hip");
    session.save_hip(&hip.to_string_lossy(), false)?;
    println!("Saving {}", hip.to_string_lossy());
    Ok(())
}
