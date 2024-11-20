#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused)]
use hapi_rs::asset::AssetLibrary;
use hapi_rs::attribute::{NumericAttr, StorageType};
use hapi_rs::enums::AttributeOwner;
use hapi_rs::geometry::extra::GeometryExtension;
use hapi_rs::geometry::AttributeInfo;
use hapi_rs::node::Geometry;
use hapi_rs::parameter::{FloatParameter, Parameter};
use hapi_rs::raw::ThriftSharedMemoryBufferType;
use hapi_rs::session::{
    connect_to_memory_server, connect_to_pipe, start_engine_pipe_server,
    start_shared_memory_server, SessionOptions, ThriftServerOptions,
};
use hapi_rs::Result;
use std::time::Duration;

const N_RUN: usize = 100;

fn copy_geo(source: &Geometry, input_geo: &Geometry) -> Result<()> {
    let part = source.part_info(0)?.unwrap();
    input_geo.set_part_info(&part)?;
    let position_attr = source.get_position_attribute(part.part_id())?;
    let positions_data = position_attr.get(0)?;
    let vertex_list = source.vertex_list(None)?;
    let face_counts = source.get_face_counts(None)?;
    input_geo.set_vertex_list(0, &vertex_list)?;
    input_geo.set_face_counts(0, &face_counts)?;

    let color_data = source
        .get_point_color_attribute(&part)?
        .expect("Cd attribute")
        .get(part.part_id())?;

    input_geo
        .create_position_attribute(&part)?
        .set(part.part_id(), &positions_data)?;
    input_geo
        .create_point_color_attribute(&part)?
        .set(part.part_id(), &color_data)?;
    input_geo.commit()?;

    Ok(())
}

fn main() -> Result<()> {
    let server_options = ThriftServerOptions::default();
    let session_options = SessionOptions::builder().threaded(false).build();
    let conn_name = "hapi-bench";
    let server_type = std::env::args().nth(1).expect("server type");
    println!("Starting \"{server_type}\" server");
    let session = match server_type.as_str() {
        "pipe" => {
            let pid = start_engine_pipe_server(conn_name, None, &server_options)?;
            connect_to_pipe(conn_name, Some(&session_options), None, Some(pid))?
        }
        "memory" => {
            let pid = start_shared_memory_server(conn_name, &server_options, None)?;
            connect_to_memory_server(conn_name, Some(&session_options), Some(pid))?
        }
        _ => panic!("Server type must be pipe or memory"),
    };
    println!("Session created.");
    let lib = AssetLibrary::from_file(
        session.clone(),
        "C:/Github/hapi-rs/benchmarks/shared-memory/benchmark.hda",
    )?;

    let asset_node = lib.try_create_first()?;
    println!("Test asset ready.");
    println!("Running {N_RUN} times");
    let start = std::time::Instant::now();
    let Ok(Parameter::Float(rotate_parm)) = asset_node.parameter("rotate") else {
        panic!("Missing rotate parameter");
    };
    for n in 0..N_RUN {
        rotate_parm.set(0, (n * 10) as f32);
        asset_node.cook()?;
        let geometry = asset_node.geometry()?.expect("geometry");
        let input = session.create_input_node(&format!("copy_{n}"), None)?;
        copy_geo(&geometry, &input)?
    }
    println!(
        "Server type \"{server_type}\" done in {} milliseconds",
        start.elapsed().as_millis()
    );
    Ok(())
}
