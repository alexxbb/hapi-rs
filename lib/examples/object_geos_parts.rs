// Translated from object_geos_parts.cpp
use hapi_rs::Result;
use hapi_rs::attribute::*;
use hapi_rs::geometry::*;
use hapi_rs::session::*;

fn main() -> Result<()> {
    let session = new_thrift_session(SessionOptions::default(), ServerOptions::shared_memory())?;

    let profile = session.start_performance_monitor_profile("hapi-rs")?;

    let lib = session.load_asset_file("../otls/sesi/SideFX_spaceship.hda")?;
    let node = lib.try_create_first()?;
    node.cook_blocking()?;
    let _asset_info = node.asset_info()?;

    for info in node.get_objects_info()? {
        let obj_node = info.to_node()?;
        let Some(geometry) = obj_node.geometry()? else {
            continue;
        };
        for part_info in geometry.partitions()? {
            println!(
                "Object: {}, Display: {}, Partition: {}",
                obj_node.path()?,
                geometry.node.path_relative(Some(obj_node.handle))?,
                part_info.part_id()
            );
            let attrib_names = geometry.get_attribute_names(AttributeOwner::Point, &part_info)?;
            println!(
                "{}",
                &attrib_names.iter_str().collect::<Vec<&str>>().join("\n")
            );
            println!("Point Positions: ");
            let attrib = geometry
                .get_attribute(part_info.part_id(), AttributeOwner::Point, "P")?
                .unwrap();

            let attrib = attrib.downcast::<NumericAttr<f32>>().unwrap();
            let positions = attrib.get(part_info.part_id())?;
            for p in 0..attrib.info().count() {
                let idx = (p * attrib.info().tuple_size()) as usize;
                println!("{:?}", &positions[idx..idx + 3])
            }

            println!("Number of Faces: {}", part_info.face_count());
            let faces = geometry.get_face_counts(&part_info)?;
            if part_info.part_type() != PartType::Curve {
                for face in faces.iter() {
                    print!("{}, ", face);
                }
                println!();
            }
            let vertices = geometry.vertex_list(&part_info)?;
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
    let tmp_file = tempfile::Builder::new()
        .suffix("perf-monitor.json")
        .tempfile()
        .expect("per-monitor temp file created");
    let (_, tmp_file) = tmp_file.keep().unwrap();
    let out_file = tmp_file.to_string_lossy();
    session.stop_performance_monitor_profile(profile, &out_file)?;
    println!("Performance monitor data saved: {}", &out_file);
    Ok(())
}
