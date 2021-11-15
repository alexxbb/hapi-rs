/// curve_marshall.cpp
use hapi_rs::{attribute::*, geometry::*, session::*};

fn main() -> Result<()> {
    let mut session = new_in_process()?;
    session.initialize(&SessionOptions::default())?;
    let new_node = session.create_input_node("Curve")?;
    new_node.cook_blocking(None)?;
    let part_info = PartInfo::default()
        .with_part_type(PartType::Curve)
        .with_face_count(1)
        .with_vertex_count(4)
        .with_point_count(4);

    let curve_info = CurveInfo::default()
        .with_curve_type(CurveType::Nurbs)
        .with_curve_count(1)
        .with_vertex_count(4)
        .with_knot_count(8)
        .with_order(4)
        .with_periodic(true)
        .with_has_knots(true);

    let geom = new_node.geometry()?.expect("geometry");
    geom.set_part_info(&part_info)?;
    geom.set_curve_info(&curve_info, 0)?;
    geom.set_curve_counts(part_info.part_id(), &[4])?;
    geom.set_curve_knots(
        part_info.part_id(),
        &[0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
    )?;

    let p_info = AttributeInfo::default()
        .with_count(4)
        .with_tuple_size(3)
        .with_storage(StorageType::Float)
        .with_owner(AttributeOwner::Point);
    let p_attrib = geom.add_attribute::<f32>("P", 0, &p_info)?;

    #[rustfmt::skip]
        p_attrib.set(
        0,
        &[
            -4.0, 0.0, 4.0,
            -4.0, 0.0, -4.0,
            4.0, 0.0, -4.0,
            4.0, 0.0, 4.0,
        ])?;
    geom.commit()?;

    session.save_hip("curve_marshall.hip", true)?;
    println!("Saving curve_marshall.hip");
    Ok(())
}
