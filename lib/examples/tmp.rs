use hapi_rs::{Result, attribute::*, geometry::*};

use hapi_rs::session::simple_session;

fn main() -> Result<()> {
    let session = simple_session()?;
    let geo = session.create_input_node("dummy", None)?;
    let part = PartInfo::default()
        .with_part_type(PartType::Mesh)
        // .with_face_count(1)
        // .with_vertex_count(1)
        .with_point_count(2);
    geo.set_part_info(&part)?;
    let p_info = AttributeInfo::default()
        .with_count(part.point_count())
        .with_tuple_size(3)
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Float);
    let p_attr = geo.add_numeric_attribute::<f32>("P", 0, p_info)?;
    p_attr.set(0, &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0])?;

    let a_attr = geo.add_numeric_attribute::<f32>(
        "a",
        0,
        AttributeInfo::default()
            .with_count(1)
            .with_tuple_size(1)
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::Float),
    )?;
    a_attr.set(0, &[1.0, 1.0])?;
    geo.commit()?;
    geo.node.cook_blocking()?;
    geo.save_to_file("/tmp/foo2.bgeo")?;
    Ok(())
}
