#![allow(dead_code)]
#![allow(unused)]

use hapi_rs::enums::StatusVerbosity;
use hapi_rs::node::{CookResult, HoudiniNode, NodeFlags, NodeHandle, NodeType};
use hapi_rs::raw::StatusType;
use hapi_rs::session::{
    connect_to_memory_server, quick_session, SessionInfo, SessionOptions, SessionOptionsBuilder,
};
use hapi_rs::Result;

const OTL: &str = "otls/hapi_errors.hda";

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
    let opts = SessionOptionsBuilder::default()
        .threaded(true)
        .log_file("c:/Temp/hapi.log.txt")
        .build();
    let session = quick_session(Some(&opts))?;
    let asset = session.load_asset_file(otl)?.try_create_first()?;
    let geo = asset.geometry()?.unwrap();

    geo.node.cook_blocking()?;
    let message_nodes = asset.get_message_nodes()?;

    let error = match &message_nodes[..] {
        [] => asset.get_composed_cook_result_string(StatusVerbosity::Statusverbosity2)?,
        message_nodes => gather_all_messages(asset, &message_nodes)?,
    };

    println!("-{}", error);
    Ok(())
}
