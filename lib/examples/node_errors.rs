#![allow(dead_code)]
#![allow(unused)]

use hapi_rs::Result;
use hapi_rs::enums::StatusVerbosity;
use hapi_rs::node::{CookResult, HoudiniNode, NodeFlags, NodeHandle, NodeType};
use hapi_rs::raw::StatusType;
use hapi_rs::server::ServerOptions;
use hapi_rs::session::{SessionOptions, new_thrift_session};

const OTL: &str = "../otls/hapi_errors.hda";

fn gather_all_messages(asset: HoudiniNode, message_nodes: &[NodeHandle]) -> Result<String> {
    let mut message = String::new();
    for handle in message_nodes {
        let node = handle.to_node(&asset.session)?;
        match node.cook_blocking()? {
            CookResult::Succeeded => {}
            CookResult::FatalErrors(_) | CookResult::CookErrors(_) => {
                let err = node.get_cook_result_string(StatusVerbosity::Statusverbosity2)?;
                message.push_str(&err);
            }
        }
    }
    Ok(message)
}

fn main() -> Result<()> {
    let otl = std::env::current_dir().unwrap().join(OTL);
    let otl = std::path::absolute(&otl).unwrap();
    let log_file = std::env::temp_dir().join("hapi.log");
    let session = new_thrift_session(
        SessionOptions::default().threaded(true),
        ServerOptions::shared_memory_with_defaults().with_log_file(&log_file),
    )?;
    let asset = session.load_asset_file(otl)?.try_create_first()?;
    let geo = asset.geometry()?.unwrap();

    geo.node.cook_blocking()?;
    let message_nodes = asset.get_message_nodes()?;
    let error = asset.get_composed_cook_result_string(StatusVerbosity::Statusverbosity2)?;

    println!("\n------ Node Errors ------{}", error);
    Ok(())
}
