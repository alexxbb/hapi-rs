use hapi_rs::geometry::{
    AttributeInfo, AttributeOwner, GroupType, PartInfo, PartType, StorageType,
};
use hapi_rs::node::HoudiniNode;
use hapi_rs::session::{new_in_process, Session, SessionOptions};
use hapi_rs::Result;

fn create_cube(session: &Session) -> Result<HoudiniNode> {
    let cube_node = session.create_input_node("Cube")?;
    cube_node.cook(None)?;

    let part_info = PartInfo::default()
        .with_part_type(PartType::Mesh)
        .with_face_count(6)
        .with_vertex_count(24)
        .with_point_count(8);

    let geometry = cube_node.geometry()?.unwrap();
    geometry.set_part_info(&part_info)?;

    let attr_info = AttributeInfo::default()
        .with_count(8)
        .with_tuple_size(3)
        .with_storage(StorageType::Float)
        .with_owner(AttributeOwner::Point);

    let p_attr = geometry.add_attribute::<f32>("P", 0, &attr_info)?;

    #[rustfmt::skip]
        let positions = [
        0.0f32, 0.0, 0.0,
        0.0, 0.0, 1.0,
        0.0, 1.0, 0.0,
        0.0, 1.0, 1.0,
        1.0, 0.0, 0.0,
        1.0, 0.0, 1.0,
        1.0, 1.0, 0.0,
        1.0, 1.0, 1.0
    ];

    p_attr.set(0, &positions)?;

    #[rustfmt::skip]
        let vertices = [
        0, 2, 6, 4,
        2, 3, 7, 6,
        2, 0, 1, 3,
        1, 5, 7, 3,
        5, 4, 6, 7,
        0, 4, 5, 1
    ];
    geometry.set_vertex_list(0, &vertices)?;
    geometry.set_face_counts(0, &[4, 4, 4, 4, 4, 4])?;

    geometry.add_group(0, "pointGroup", GroupType::Point)?;
    let num_elem = part_info.element_count_by_group(GroupType::Point);
    let membership: Vec<_> = (0..num_elem)
        .map(|v| if (v % 2) > 0 { 1 } else { 0 })
        .collect();
    geometry.set_group_membership(
        part_info.part_id(),
        GroupType::Point,
        "pointGroup",
        &membership,
    )?;
    geometry.commit()?;
    Ok(cube_node)
}

fn main() -> Result<()> {
    let mut session = new_in_process()?;
    session.initialize(&SessionOptions::default())?;
    let cube = create_cube(&session)?;
    let xform = session.create_node("Sop/xform", Some("PointGroupManipulator"), None)?;
    cube.connect_input(0, &xform, 0)?;
    session.save_hip("/tmp/bla.hip", true)?;
    Ok(())
}
