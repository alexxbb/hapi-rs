use crate::ffi::{
    raw::{PdgEventType, PdgState},
    PDGEventInfo, PDGWorkItemInfo, PDGWorkItemResult,
};
use crate::node::HoudiniNode;
use crate::Result;

pub struct PDGWorkItem<'session> {
    pub info: PDGWorkItemInfo,
    pub id: i32,
    pub context_id: i32,
    pub node: &'session HoudiniNode,
}

impl<'session> PDGWorkItem<'session> {
    pub fn get_results(&self) -> Result<Vec<PDGWorkItemResult>> {
        crate::ffi::get_workitem_result(
            &self.node.session,
            self.node.handle,
            self.id,
            self.info.num_results(),
        )
        .map(|results| {
            results
                .into_iter()
                .map(|result| PDGWorkItemResult { inner: result })
                .collect()
        })
    }
}

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

    pub fn get_current_state(&self, context_id: Option<i32>) -> Result<PdgState> {
        let context = match context_id {
            Some(c) => c,
            None => self.get_context_id()?,
        };
        crate::ffi::get_pdg_state(&self.node.session, context)
    }

    pub fn get_workitem(
        &self,
        workitem_id: i32,
        context_id: Option<i32>,
    ) -> Result<PDGWorkItem<'_>> {
        let context_id = match context_id {
            Some(c) => c,
            None => self.get_context_id()?,
        };
        crate::ffi::get_workitem_info(&self.node.session, context_id, workitem_id).map(|inner| {
            PDGWorkItem {
                info: PDGWorkItemInfo { inner },
                id: workitem_id,
                context_id,
                node: &self.node,
            }
        })
    }
}
