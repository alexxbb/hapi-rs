// Translated from object_geos_parts.cpp
use hapi_rs::geometry::AttributeOwner;
use hapi_rs::session::*;
use hapi_rs::{PartType, Result};

fn main() -> Result<()> {
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
                let attrib_names =
                    geometry.get_attribute_names(AttributeOwner::Point, Some(&part_info))?;
                println!(
                    "{}",
                    &attrib_names.iter_str().collect::<Vec<&str>>().join("\n")
                );
                println!("Point Positions: ");
                let attrib = geometry
                    .get_attribute::<f32>(part_info.part_id(), AttributeOwner::Point, "P")?
                    .unwrap();

                let positions = attrib.read(part_info.part_id())?;
                for p in 0..attrib.info.count() {
                    let idx = (p * attrib.info.tuple_size()) as usize;
                    println!("{:?}", &positions[idx..idx + 3])
                }

                println!("Number of Faces: {}", part_info.face_count());
                let faces = geometry.get_face_counts(Some(&part_info))?;
                if part_info.part_type() != PartType::Curve {
                    for face in faces.iter() {
                        print!("{}, ", face);
                    }
                    println!();
                }
                let vertices = geometry.vertex_list(Some(&part_info))?;
                println!("Vertex Indices Into Points Array");
                let mut curr_idx = 0;
                assert!(curr_idx < vertices.len());
                for (face, count) in faces.iter().enumerate() {
                    for _ in 0..(*count as usize) {
                        println!(
                            "Vertex: {0}, \
                                  belonging to face: {1}, \
                                  index: {2} of point array ",
                            curr_idx, face, vertices[curr_idx]
                        );
                        curr_idx += 1;
                    }
                }
            }
        }
    }

    Ok(())
}
