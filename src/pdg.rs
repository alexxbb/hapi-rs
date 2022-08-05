use crate::ffi::{
    raw::{PdgEventType, PdgState},
    PDGEventInfo, PDGWorkItemInfo,
};
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

    pub fn get_context_id(&self) -> Result<i32> {
        crate::ffi::get_pdg_context_id(&self.node.session, self.node.handle)
    }

    pub fn cancel_cook(&self) -> Result<()> {
        let context = self.get_context_id()?;
        crate::ffi::cancel_pdg_cook(&self.node.session, context)
    }

    pub fn dirty_pdg_node(&self, clean_results: bool) -> Result<()> {
        crate::ffi::dirty_pdg_node(&self.node.session, self.node.handle, clean_results)
    }

    pub fn get_current_state(&self) -> Result<PdgState> {
        let context = self.get_context_id()?;
        crate::ffi::get_pdg_state(&self.node.session, context)
    }

    pub fn get_workitem_info(&self, workitem_id: i32) -> Result<PDGWorkItemInfo> {
        let context_id = self.get_context_id()?;
        crate::ffi::get_workitem_info(&self.node.session, context_id, workitem_id)
            .map(|inner| PDGWorkItemInfo { inner })
    }
}
