use crate::ffi;
use crate::ffi::{
    raw::{PdgEventType, PdgState},
    PDGEventInfo, PDGWorkItemInfo, PDGWorkItemResult,
};
use crate::node::HoudiniNode;
use crate::Result;
use std::fmt::Formatter;
use std::ops::ControlFlow;

pub struct PDGWorkItem<'session> {
    pub info: PDGWorkItemInfo,
    pub id: i32,
    pub context_id: i32,
    pub node: &'session HoudiniNode,
}

impl std::fmt::Debug for PDGWorkItem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PDGWorkItem")
            .field("id", &self.id)
            .field("context", &self.context_id)
            .field("name", &self.info.name(&self.node.session))
            .finish()
    }
}

impl<'session> PDGWorkItem<'session> {
    pub fn get_results(&self) -> Result<Vec<PDGWorkItemResult>> {
        ffi::get_workitem_result(
            &self.node.session,
            self.node.handle,
            self.id,
            self.info.output_file_count(),
        )
        .map(|results| {
            results
                .into_iter()
                .map(|result| PDGWorkItemResult { inner: result })
                .collect()
        })
    }
}

#[derive(Debug, Clone)]
pub struct TopNode {
    pub node: HoudiniNode,
}

impl TopNode {
    pub fn cook<F>(&self, mut func: F) -> Result<()>
    where
        F: FnMut(PDGEventInfo, i32) -> ControlFlow<bool>,
    {
        ffi::cook_pdg(&self.node.session, self.node.handle, false)?;
        'main: loop {
            let (graph_ids, graph_names) = ffi::get_pdg_contexts(&self.node.session)?;
            for (ctx_id, ctx_name) in graph_ids.into_iter().zip(graph_names) {
                for event in ffi::get_pdg_events(&self.node.session, ctx_id)? {
                    let event = PDGEventInfo { inner: event };
                    match event.event_type() {
                        PdgEventType::EventCookComplete => break 'main,
                        _ => {
                            if let ControlFlow::Break(cancel_cook) = func(event, ctx_name) {
                                if cancel_cook {
                                    // TODO: Should we call this for all graph ids?
                                    ffi::cancel_pdg_cook(&self.node.session, ctx_id)?;
                                }
                                break 'main;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_context_id(&self) -> Result<i32> {
        ffi::get_pdg_context_id(&self.node.session, self.node.handle)
    }

    pub fn cancel_cook(&self) -> Result<()> {
        let context = self.get_context_id()?;
        ffi::cancel_pdg_cook(&self.node.session, context)
    }

    pub fn dirty_node(&self, clean_results: bool) -> Result<()> {
        ffi::dirty_pdg_node(&self.node.session, self.node.handle, clean_results)
    }

    pub fn get_current_state(&self, context_id: Option<i32>) -> Result<PdgState> {
        let context = match context_id {
            Some(c) => c,
            None => self.get_context_id()?,
        };
        ffi::get_pdg_state(&self.node.session, context)
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
        ffi::get_workitem_info(&self.node.session, context_id, workitem_id).map(|inner| {
            PDGWorkItem {
                info: PDGWorkItemInfo { inner },
                id: workitem_id,
                context_id,
                node: &self.node,
            }
        })
    }

    pub fn get_workitems(&self) -> Result<Vec<PDGWorkItem<'_>>> {
        let context_id = self.get_context_id()?;
        ffi::get_pdg_workitems(&self.node.session, self.node.handle)?
            .into_iter()
            .map(|workitem_id| {
                Ok(PDGWorkItem {
                    info: PDGWorkItemInfo {
                        inner: ffi::get_workitem_info(&self.node.session, context_id, workitem_id)?,
                    },
                    id: workitem_id,
                    context_id,
                    node: &self.node,
                })
            })
            .collect()
    }
}
