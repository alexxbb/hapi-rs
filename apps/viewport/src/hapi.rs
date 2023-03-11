#![allow(unused)]
mod hapi_geo;

use hapi_rs::attribute::NumericAttr;
use hapi_rs::enums::AttributeOwner;
use hapi_rs::session::{connect_to_pipe, quick_session};
use hapi_rs::Result;
use std::iter::once;
use ultraviolet::{Vec2, Vec3};

static OTL: &str = r#"C:\Github\hapi-rs\apps\viewport\otls\hapi_cube.hda"#;

fn main() -> Result<()> {
    // let session = quick_session(None)?;
    let session = connect_to_pipe("hapi", None, None)?;
    let lib = session.load_asset_file(OTL)?;
    let asset = lib.try_create_first()?;
    let geo = asset.geometry()?.expect("Geometry");
    geo.node.cook()?;

    // let positions = geo.get_position_attribute(0)?.get(0)?;
    // let uv_attr = geo
    //     .get_attribute(0, AttributeOwner::Vertex, "uv")?
    //     .expect("uv attribute");
    // let uv_attr = uv_attr
    //     .downcast::<NumericAttr<f32>>()
    //     .unwrap()
    //     .get(0)
    //     .unwrap();
    // let n_attr = geo
    //     .get_attribute(0, AttributeOwner::Vertex, "N")?
    //     .expect("N attribute");
    // let n_attr = n_attr
    //     .downcast::<NumericAttr<f32>>()
    //     .unwrap()
    //     .get(0)
    //     .unwrap();
    // let vertex_list = geo.vertex_list(None)?;
    // let face_counts = geo.get_face_counts(None)?;

    // dbg!(&positions.len() / 3);
    // dbg!(&uv_attr.len() / 3);
    // dbg!(face_counts);

    let mesh = hapi_geo::MeshData::from_houdini_geo(&geo)?;
    dbg!(&mesh.vertex_array);

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
