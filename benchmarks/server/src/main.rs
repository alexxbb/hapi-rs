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
use hapi_rs::raw::{ThriftSharedMemoryBufferType, HAPI_PRIM_MIN_VERTEX_COUNT};
use hapi_rs::server::ThriftTransport;
use hapi_rs::server::{ServerOptions, ThriftSharedMemoryTransportBuilder};
use hapi_rs::session::{new_in_process_session, new_thrift_session, SessionOptions};
use hapi_rs::Result;
use nanorand::{Rng, WyRand};
use std::ffi::CStr;
use std::time::Duration;

use std::sync::{LazyLock, Mutex};

static RNG: LazyLock<Mutex<WyRand>> = LazyLock::new(|| Mutex::new(WyRand::new()));

fn random_string(length: usize) -> String {
    let mut rng = RNG.lock().unwrap();
    let next = || rng.generate_range(98..122) as u8 as char;
    std::iter::repeat_with(next).take(length).collect()
}

const NUM_ITERATIONS: usize = 1;
const NUM_COPIES: usize = 10;
const WORKLOAD_MULTIPLIER: usize = 1;
const STR_LENGTH: usize = 10;
const BUFFER_SIZE: usize = 5_000; // GB

fn copy_geo(source: &Geometry, input_geo: &Geometry) -> Result<()> {
    let part = source.part_info(0)?;
    input_geo.set_part_info(&part)?;
    // dbg!(&part);
    let position_attr = source.get_position_attribute(&part)?;
    let positions_data = position_attr.unwrap().get(0)?;
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

    let sdf_attr = source
        .get_attribute(part.part_id(), AttributeOwner::Point, "sdf")?
        .expect("sdf attribute");
    let sdf_attr = sdf_attr
        .downcast::<NumericAttr<f32>>()
        .expect("sdf is float type");
    let sdf_data = sdf_attr.get(part.part_id())?;

    input_geo
        .create_position_attribute(&part)?
        .set(part.part_id(), &positions_data)?;
    let dest_payload_attr =
        input_geo.add_string_attribute("payload", part.part_id(), payload_attr.info().clone())?;
    dest_payload_attr.set(part.part_id(), &payload_c_strings)?;
    let dest_sdf_attr =
        input_geo.add_numeric_attribute::<f32>("sdf", part.part_id(), sdf_attr.info().clone())?;
    dest_sdf_attr.set(part.part_id(), &sdf_data)?;

    let test_attr = input_geo.add_numeric_attribute::<f32>(
        "test",
        part.part_id(),
        AttributeInfo::default()
            .with_count(part.point_count())
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::Float)
            .with_tuple_size(1),
    )?;
    test_attr.set(part.part_id(), &[0.1, 0.2, 0.3])?;
    input_geo.commit()?;
    // input_geo.node.cook_blocking()?;
    // input_geo.save_to_file("/tmp/foo.bgeo")?;

    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args();
    let session_options = SessionOptions::default().threaded(false);
    let conn_name = format!("hapi-bench-{}", random_string(3));
    let server_type = args.nth(1).expect("server type");
    let workload_multiplier = match args.next() {
        None => WORKLOAD_MULTIPLIER,
        Some(v) => v
            .parse::<usize>()
            .expect("Workload multiplier must be a positive number"),
    };

    let num_iterations = NUM_ITERATIONS * workload_multiplier;
    let num_copies = NUM_COPIES * workload_multiplier;
    let str_length = STR_LENGTH * workload_multiplier;

    println!("Starting \"{server_type}\" server");
    let server_options = match server_type.as_str() {
        "pipe" => ServerOptions::pipe_with_defaults(),
        "memory-fixed" | "memory-ring" => {
            let buffer_type = match server_type.as_str() {
                "memory-fixed" => ThriftSharedMemoryBufferType::Buffer,
                "memory-ring" => ThriftSharedMemoryBufferType::RingBuffer,
                _ => unreachable!(),
            };
            ServerOptions::shared_memory_with_defaults().with_thrift_transport(
                ThriftTransport::SharedMemory(
                    ThriftSharedMemoryTransportBuilder::default()
                        .with_buffer_type(buffer_type)
                        .with_memory_name(conn_name)
                        .with_buffer_size(BUFFER_SIZE as i64) // MB
                        .build(),
                ),
            )
        }
        _ => panic!("Server type must be pipe or memory-fixed or memory-ring"),
    }
    .with_auto_close(true);
    let session = new_thrift_session(session_options, server_options)?;
    println!("Session created.");
    let asset_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmark.hda");
    let lib = AssetLibrary::from_file(session.clone(), &asset_path)?;

    let asset_node = lib.try_create_first()?;
    let start = std::time::Instant::now();
    let Ok(Parameter::Float(anim_param)) = asset_node.parameter("anim") else {
        panic!("Missing anim parameter");
    };

    let Ok(Parameter::String(point_string_param)) = asset_node.parameter("point_string") else {
        panic!("Missing point_string parameter");
    };

    if let Ok(Parameter::Int(num_copies_param)) = asset_node.parameter("num_copies") {
        num_copies_param.set(0, num_copies as i32)?;
    }
    println!("Running benchmark with {num_iterations} iterations, {num_copies} copies, {str_length} string length");
    let mut total_time = Duration::from_millis(0);
    for n in 0..num_iterations {
        anim_param.set(0, (n * 10) as f32);
        point_string_param.set(0, random_string(str_length))?;
        // TODO: substruct cooking time from total time?
        asset_node.cook()?;
        let geometry = asset_node.geometry()?.expect("geometry");
        let input = session.create_input_node(&format!("copy_{n}"), None)?;
        let copy_start = std::time::Instant::now();
        copy_geo(&geometry, &input)?;
        total_time += copy_start.elapsed();
        println!(
            " - iteration {}/{num_iterations} done in {:.2?} seconds",
            n + 1,
            copy_start.elapsed().as_secs_f32()
        );
    }
    let avg_time = total_time.div_f32(num_iterations as f32).as_secs_f32();
    println!("Server type \"{server_type}\" done in {avg_time:.2} seconds (avg)",);
    // session.save_hip("c:/Temp/junk/bench.hip", true)?;
    Ok(())
}
