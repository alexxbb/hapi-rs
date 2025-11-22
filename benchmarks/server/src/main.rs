#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused)]

use hapi_rs::asset::AssetLibrary;
use hapi_rs::attribute::{AsAttribute, NumericAttr, StorageType, StringAttr};
use hapi_rs::enums::AttributeOwner;
use hapi_rs::geometry::extra::GeometryExtension;
use hapi_rs::geometry::AttributeInfo;
use hapi_rs::node::Geometry;
use hapi_rs::parameter::{FloatParameter, Parameter};
use hapi_rs::raw::ThriftSharedMemoryBufferType;
use hapi_rs::server::ServerOptions;
use hapi_rs::session::{new_in_process_session, new_thrift_session, SessionOptions};
use hapi_rs::Result;
use nanorand::{Rng, WyRand};
use std::ffi::CStr;
use std::time::Duration;

use std::sync::{LazyLock, Mutex};

static RNG: LazyLock<Mutex<WyRand>> = LazyLock::new(|| Mutex::new(WyRand::new()));

fn random_string<const SIZE: usize>() -> String {
    let mut rng = RNG.lock().unwrap();
    let next = || rng.generate_range(98..122) as u8 as char;
    std::iter::repeat_with(next).take(SIZE).collect()
}

const N_RUN: usize = 10;

fn copy_geo(source: &Geometry, input_geo: &Geometry) -> Result<()> {
    let part = source.part_info(0)?;
    input_geo.set_part_info(&part)?;
    let position_attr = source.get_position_attribute(part.part_id())?;
    let positions_data = position_attr.get(0)?;
    let vertex_list = source.vertex_list(&part)?;
    let face_counts = source.get_face_counts(&part)?;
    input_geo.set_vertex_list(0, &vertex_list)?;
    input_geo.set_face_counts(0, &face_counts)?;

    let payload_attr = source
        .get_attribute(part.part_id(), AttributeOwner::Point, "payload")?
        .expect("Geometry to have payload string attribute");

    let payload_attr = payload_attr
        .downcast::<StringAttr>()
        .expect("payload is string type");
    let payload_data = payload_attr.get(part.part_id())?;

    let payload_c_strings: Vec<&CStr> = payload_data.iter_cstr().collect();

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
    let dest_payload_attr =
        input_geo.add_string_attribute("payload", part.part_id(), payload_attr.info().clone())?;
    dest_payload_attr.set(part.part_id(), &payload_c_strings)?;
    input_geo.commit()?;

    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args();
    let session_options = SessionOptions::default().threaded(false);
    let conn_name = "hapi-bench";
    let server_type = args.nth(1).expect("server type");
    let num_runs: usize = match args.next() {
        None => N_RUN,
        Some(v) => v.parse().expect("Second argument must be a number"),
    };
    println!("Starting \"{server_type}\" server");
    let server_options = match server_type.as_str() {
        "pipe" => ServerOptions::pipe(conn_name),
        "memory" => ServerOptions::shared_memory()
            .memory_name(conn_name)
            .configure_thrift(|opts| {
                opts.set_shared_memory_buffer_type(ThriftSharedMemoryBufferType::Buffer);
                opts.set_shared_memory_buffer_size(1000);
            }),
        "in-process" => ServerOptions::shared_memory(), // not used
        _ => panic!("Server type must be pipe or memory"),
    };
    let session = match server_type.as_str() {
        "pipe" | "memory" => new_thrift_session(session_options, server_options)?,
        "in-process" => new_in_process_session(None)?,
        _ => panic!("Server type must be one of pipe, memory, or in-process"),
    };
    println!("Session created.");
    let asset_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmark.hda");
    let lib = AssetLibrary::from_file(session.clone(), &asset_path)?;

    let asset_node = lib.try_create_first()?;
    println!("Test asset ready.");
    println!("Running {num_runs} times");
    let start = std::time::Instant::now();
    let Ok(Parameter::Float(rotate_parm)) = asset_node.parameter("rotate") else {
        panic!("Missing rotate parameter");
    };

    let Ok(Parameter::String(input_str)) = asset_node.parameter("string") else {
        panic!("Missing string parameter");
    };
    for n in 0..num_runs {
        rotate_parm.set(0, (n * 10) as f32);
        input_str.set(0, random_string::<10>())?;
        asset_node.cook()?;
        let geometry = asset_node.geometry()?.expect("geometry");
        let input = session.create_input_node(&format!("copy_{n}"), None)?;
        copy_geo(&geometry, &input)?
    }
    println!(
        "Server type \"{server_type}\" done in {} milliseconds",
        start.elapsed().as_millis()
    );
    // session.save_hip("c:/Temp/junk/bench.hip", true)?;
    Ok(())
}
