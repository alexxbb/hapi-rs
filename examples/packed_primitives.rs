#![allow(dead_code)]
#![allow(unused)]

use hapi_rs::geometry::{PackedPrimInstancingMode as IM, *};
use hapi_rs::node::*;
use hapi_rs::session::*;
use hapi_rs::Result;

fn main() -> Result<()> {
    let session = quick_session(None)?;

    let lib = session.load_asset_file("otls/sesi/PackedPrimitive.hda")?;
    let asset = lib.try_create_first()?;
    let mut co = CookOptions::default();
    for mode in [IM::Disabled, IM::Hierarchy, IM::Flat] {
        println!(
            "Using PackedPrimInstancingMode::{}",
            match mode {
                PackedPrimInstancingMode::Disabled => "Disabled",
                PackedPrimInstancingMode::Hierarchy => "Hierarchy",
                PackedPrimInstancingMode::Flat => "Flat",
                _ => unreachable!(),
            }
        );
        co.set_packed_prim_instancing_mode(mode);
        asset.cook_blocking(Some(&co))?;

        let nodes = asset.find_children_by_type(NodeType::Sop, NodeFlags::Any, false)?;
        for handle in nodes {
            let node = handle.to_node(&session)?;
            node.cook_blocking(Some(&co))?;
            let geo = node.geometry()?.expect("geometry");
            println!(
                "Part count for node {:?}: {}",
                geo.node,
                geo.geo_info()?.part_count()
            );
            for part in geo.partitions()? {
                println!(
                    "Part {}\n   Point Count = {}\n   Type = {}",
                    part.part_id(),
                    part.point_count(),
                    match part.part_type() {
                        PartType::Mesh => "Mesh",
                        PartType::Curve => "Curve",
                        PartType::Instancer => "Instancer",
                        p => "oops",
                    }
                );
            }
        }
    }
    Ok(())
}
