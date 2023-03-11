#![allow(unused)]
mod hapi_geo;

use std::iter::once;
use hapi_rs::attribute::NumericAttr;
use hapi_rs::enums::AttributeOwner;
use hapi_rs::session::{quick_session, connect_to_pipe};
use hapi_rs::Result;
use ultraviolet::{Vec3, Vec2};

static OTL: &str = r#"C:\Github\hapi-rs\apps\viewport\otls\hapi_cube.hda"#;

fn main() -> Result<()> {
    // let session = quick_session(None)?;
    let session = connect_to_pipe("hapi", None, None)?;
    let lib = session.load_asset_file(OTL)?;
    let asset = lib.try_create_first()?;
    let geo = asset.geometry()?.expect("Geometry");
    geo.node.cook()?;

    let positions = geo.get_position_attribute(0)?.get(0)?;
    let uv_attr = geo.get_attribute(0, AttributeOwner::Vertex, "uv")?.expect("uv attribute");
    let uv_attr = uv_attr.downcast::<NumericAttr<f32>>().unwrap().get(0).unwrap();
    let n_attr = geo.get_attribute(0, AttributeOwner::Vertex, "N")?.expect("N attribute");
    let n_attr = n_attr.downcast::<NumericAttr<f32>>().unwrap().get(0).unwrap();
    let vertex_list = geo.vertex_list(None)?;
    let face_counts = geo.get_face_counts(None)?;

    dbg!(&positions.len() / 3);
    dbg!(&uv_attr.len() / 3);
    // dbg!(face_counts);



    let mut buffer = Vec::new();

    #[derive(Clone, Debug)]
    struct Vertex{
        pos: Vec3,
        uv: Vec2,
    }

    let mut offset = 0;
    for vertex_count_per_face in &face_counts {
        let num_triangles = (vertex_count_per_face - 2) as usize;
        for i in 0..num_triangles {

            let off0 = offset + 0;
            let off1 = offset + i + 1;
            let off2 = offset + i + 2;

            let tri_a = vertex_list[off0] as usize;
            let tri_b = vertex_list[off1] as usize;
            let tri_c = vertex_list[off2] as usize;


            let pos_a = Vec3::new(
                         positions[tri_a * 3 + 0],
                        positions[tri_a * 3 + 1],
                        positions[tri_a * 3 + 2]
            );
            let pos_b = Vec3::new(
                positions[tri_b * 3 + 0],
                positions[tri_b * 3 + 1],
                positions[tri_b * 3 + 2]
            );
            let pos_c = Vec3::new(
                positions[tri_c * 3 + 0],
                positions[tri_c * 3 + 1],
                positions[tri_c * 3 + 2]
            );
            let uv_a = Vec2::new(uv_attr[off0 * 3 + 0], uv_attr[off0 * 3 + 1]);
            let uv_b = Vec2::new(uv_attr[off1 * 3 + 0], uv_attr[off1 * 3 + 1]);
            let uv_c = Vec2::new(uv_attr[off2 * 3 + 0], uv_attr[off2 * 3 + 1]);

            buffer.extend_from_slice(&[
                Vertex {
                    pos: pos_a,
                    uv: uv_a,
                },
                Vertex {
                    pos: pos_b,
                    uv: uv_b,
                },
                Vertex {
                    pos: pos_c,
                    uv: uv_c,
                },
            ]);


        }

        dbg!(&buffer);
    }



















    // let mut vertices = Vec::new();
    // let mut indices: Vec<i32> = Vec::new();

    // for (f, face_vtx_count ) in face_counts.into_iter().enumerate() {
    //     let vtx_number = f * face_vtx_count as usize;
    //     let end = vtx_number + face_vtx_count as usize;
    //     let face_point_indices = &vertex_list[vtx_number..end];
    //     println!("Face {f} vertices {:?}", face_point_indices);
    //     for (i, point_number) in face_point_indices.iter().enumerate() {
    //         let vtx_number = vtx_number + i;
    //         let stride = *point_number as usize * 3;
    //         let point = &positions[stride..stride + 3];
    //         let cur_index = vertices.len();
    //         println!(" Vt{i} - {cur_index}");
    //         vertices.push(point);
    //     }
    // }
    // // println!("{:?}", positions);
    // println!("{:?}", vertices);

    Ok(())

}