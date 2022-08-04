use crate::ffi::{raw::PdgEventType, PDGEventInfo};
use crate::node::HoudiniNode;
use crate::Result;

pub struct PDGNode {
    pub node: HoudiniNode,
}

impl PDGNode {
    pub fn cook<F: FnMut(PDGEventInfo, &str)>(&self, mut func: F) -> Result<()> {
        crate::ffi::cook_pdg(&self.node.session, self.node.handle, false)?;
        'main: loop {
            let (graph_ids, graph_names) = crate::ffi::get_pdg_contexts(&self.node.session)?;

            for (ctx_id, ctx_name) in graph_ids.into_iter().zip(graph_names) {
                let ctx_name = crate::stringhandle::get_string(ctx_name, &self.node.session)?;
                let events = crate::ffi::get_pdg_events(&self.node.session, ctx_id)?;
                for event in events {
                    let event = PDGEventInfo { inner: event };
                    match event.event_type() {
                        PdgEventType::EventCookComplete => break 'main,
                        _ => func(event, ctx_name.as_ref()),
                    }
                }
            }
        }
        Ok(())
    }
}
