use crate::ffi;
use crate::ffi::{
    raw::{PdgEventType, PdgState},
    PDGEventInfo, PDGWorkItemInfo, PDGWorkItemOutputFile,
};
use crate::node::{HoudiniNode, NodeHandle};
use crate::Result;
use std::fmt::Formatter;
use std::ops::ControlFlow;

/// Represents a single work item.
pub struct PDGWorkItem<'node> {
    pub id: WorkItemId,
    pub context_id: i32,
    pub node: &'node HoudiniNode,
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct WorkItemId(pub(crate) i32);

impl std::fmt::Debug for PDGWorkItem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PDGWorkItem")
            .field("id", &self.id)
            .field("context", &self.context_id)
            .finish()
    }
}

impl From<WorkItemId> for i32 {
    fn from(value: WorkItemId) -> Self {
        value.0
    }
}

impl From<TopNode> for NodeHandle {
    fn from(value: TopNode) -> Self {
        value.node.handle
    }
}

impl<'session> PDGWorkItem<'session> {
    pub fn get_info(&self) -> Result<PDGWorkItemInfo> {
        ffi::get_workitem_info(&self.node.session, self.context_id, self.id.0).map(PDGWorkItemInfo)
    }
    /// Retrieve the results of work, if the work item has any.
    pub fn get_results(&self) -> Result<Vec<PDGWorkItemOutputFile<'session>>> {
        match self.get_info()?.output_file_count() {
            0 => Ok(Vec::new()),
            count => {
                let results = ffi::get_workitem_result(
                    &self.node.session,
                    self.node.handle,
                    self.id.0,
                    count,
                )?;
                let results = results
                    .into_iter()
                    .map(|inner| PDGWorkItemOutputFile(inner, (&self.node.session).into()))
                    .collect();

                Ok(results)
            }
        }
    }

    pub fn get_data_length(&self, data_name: &str) -> Result<i32> {
        let data_name = std::ffi::CString::new(data_name)?;
        ffi::get_workitem_data_length(&self.node.session, self.node.handle, self.id.0, &data_name)
    }

    pub fn set_int_data(&self, data_name: &str, data: &[i32]) -> Result<()> {
        let data_name = std::ffi::CString::new(data_name)?;
        ffi::set_workitem_int_data(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &data_name,
            data,
        )
    }

    pub fn get_int_data(&self, data_name: &str) -> Result<Vec<i32>> {
        let data_name = std::ffi::CString::new(data_name)?;
        let data_size = ffi::get_workitem_data_length(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &data_name,
        )?;
        let mut buffer = Vec::new();
        buffer.resize(data_size as usize, 0);
        ffi::get_workitem_int_data(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &data_name,
            buffer.as_mut_slice(),
        )?;
        Ok(buffer)
    }

    pub fn set_float_data(&self, data_name: &str, data: &[f32]) -> Result<()> {
        let data_name = std::ffi::CString::new(data_name)?;
        ffi::set_workitem_float_data(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &data_name,
            data,
        )
    }

    pub fn get_float_data(&self, data_name: &str) -> Result<Vec<f32>> {
        let data_name = std::ffi::CString::new(data_name)?;
        let data_size = ffi::get_workitem_data_length(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &data_name,
        )?;
        let mut buffer = Vec::new();
        buffer.resize(data_size as usize, 0.0);
        ffi::get_workitem_float_data(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &data_name,
            buffer.as_mut_slice(),
        )?;
        Ok(buffer)
    }

    pub fn set_int_attribute(&self, attrib_name: &str, value: &[i32]) -> Result<()> {
        let attrib_name = std::ffi::CString::new(attrib_name)?;
        ffi::set_workitem_int_attribute(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &attrib_name,
            value,
        )
    }
    pub fn get_int_attribute(&self, attr_name: &str) -> Result<Vec<i32>> {
        let attr_name = std::ffi::CString::new(attr_name)?;
        let attr_size = ffi::get_workitem_attribute_size(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &attr_name,
        )?;
        let mut buffer = Vec::new();
        buffer.resize(attr_size as usize, 0);
        ffi::get_workitem_int_attribute(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &attr_name,
            &mut buffer,
        )?;
        Ok(buffer)
    }

    pub fn set_float_attribute(&self, attrib_name: &str, value: &[f32]) -> Result<()> {
        let attrib_name = std::ffi::CString::new(attrib_name)?;
        ffi::set_workitem_float_attribute(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &attrib_name,
            value,
        )
    }

    pub fn get_float_attribute(&self, attr_name: &str) -> Result<Vec<f32>> {
        let attr_name = std::ffi::CString::new(attr_name)?;
        let attr_size = ffi::get_workitem_attribute_size(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &attr_name,
        )?;
        let mut buffer = Vec::new();
        buffer.resize(attr_size as usize, 0.0);
        ffi::get_workitem_float_attribute(
            &self.node.session,
            self.node.handle,
            self.id.0,
            &attr_name,
            &mut buffer,
        )?;
        Ok(buffer)
    }
}

#[derive(Debug, Clone)]
/// A wrapper for [`HoudiniNode`] with methods for cooking PDG.
pub struct TopNode {
    pub node: HoudiniNode,
}

/// A convenient wrapper for a single event generated by PDG.
#[derive(Debug, Copy, Clone)]
pub struct CookStep {
    pub event: PDGEventInfo,
    pub graph_id: i32,
    pub graph_name: i32,
}

// Helper to create a vec of events. No Default impl for it.
fn create_events() -> Vec<ffi::raw::HAPI_PDG_EventInfo> {
    const NUM: usize = 32;
    vec![
        ffi::raw::HAPI_PDG_EventInfo {
            nodeId: -1,
            workItemId: -1,
            dependencyId: -1,
            currentState: -1,
            lastState: -1,
            eventType: -1,
            msgSH: -1,
        };
        NUM
    ]
}

impl TopNode {
    /// Start cooking a TOP node asynchronously.
    /// For each generated event, a user closure will be called with a [`CookStep`] argument.
    ///
    /// The closure returns [`Result<ControlFlow<bool>>`] which is handled like this:
    ///
    /// If its an `Err(_)` - bubble up the error.
    /// If it's [`ControlFlow::Break(bool)`] then the `bool` is either to cancel the cooking
    /// or just break the loop and return.
    /// In case of [`ControlFlow::Continue(_)`] run until completion.
    ///
    /// See the `pdg_cook` example in the `/examples` folder.
    pub fn cook_async<F>(&self, all_outputs: bool, mut func: F) -> Result<()>
    where
        F: FnMut(CookStep) -> Result<ControlFlow<bool>>,
    {
        let session = &self.node.session;
        log::debug!("Start cooking PDG node: {}", self.node.path()?);
        debug_assert!(session.is_valid());
        ffi::cook_pdg(session, self.node.handle, false, false, all_outputs)?;
        let mut events = create_events();
        'main: loop {
            let (graph_ids, graph_names) = ffi::get_pdg_contexts(session)?;
            debug_assert_eq!(graph_ids.len(), graph_names.len());
            for (graph_id, graph_name) in graph_ids.into_iter().zip(graph_names) {
                for event in ffi::get_pdg_events(session, graph_id, &mut events)? {
                    let event = PDGEventInfo(*event);
                    match event.event_type() {
                        PdgEventType::EventCookComplete => break 'main,
                        _ => {
                            match func(CookStep {
                                event,
                                graph_id,
                                graph_name,
                            }) {
                                Err(e) => return Err(e),
                                Ok(ControlFlow::Continue(_)) => {}
                                Ok(ControlFlow::Break(stop_cooking)) => {
                                    if stop_cooking {
                                        // TODO: Should we call this for all graph ids?
                                        ffi::cancel_pdg_cook(session, graph_id)?;
                                    }
                                    break 'main;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Trigger PDG cooking and wait for completion.
    /// If all_outputs is true and this TOP node is of topnet type, cook all network outptus.
    /// Results can then be retrieved from workitems with get_all_workitems()
    pub fn cook_pdg_blocking(&self, all_outputs: bool) -> Result<()> {
        ffi::cook_pdg(
            &self.node.session,
            self.node.handle,
            false,
            true,
            all_outputs,
        )
    }

    /// Get the graph(context) id of this node in PDG.
    pub fn get_context_id(&self) -> Result<i32> {
        ffi::get_pdg_context_id(&self.node.session, self.node.handle)
    }

    /// Cancel cooking.
    pub fn cancel_cooking(&self) -> Result<()> {
        log::debug!("Cancel PDG cooking {}", self.node.path()?);
        let context = self.get_context_id()?;
        ffi::cancel_pdg_cook(&self.node.session, context)
    }

    /// Pause cooking process
    pub fn pause_cooking(&self) -> Result<()> {
        log::debug!("Pause PDG cooking {}", self.node.path()?);
        let context = self.get_context_id()?;
        ffi::pause_pdg_cook(&self.node.session, context)
    }

    /// Dirty the node, forcing the work items to regenerate.
    pub fn dirty_node(&self, clean_results: bool) -> Result<()> {
        log::debug!("Set PDG node dirty {}", self.node.path()?);
        ffi::dirty_pdg_node(&self.node.session, self.node.handle, clean_results)
    }

    /// Which this node current [`PdgState`]
    pub fn get_current_state(&self, context_id: Option<i32>) -> Result<PdgState> {
        let context = match context_id {
            Some(c) => c,
            None => self.get_context_id()?,
        };
        ffi::get_pdg_state(&self.node.session, context)
    }

    /// Get the work item by id and graph(context) id.
    pub fn get_workitem(&self, workitem_id: WorkItemId) -> Result<PDGWorkItem<'_>> {
        let context_id = self.get_context_id()?;
        ffi::get_workitem_info(&self.node.session, context_id, workitem_id.0).map(|_| PDGWorkItem {
            id: workitem_id,
            context_id,
            node: &self.node,
        })
    }

    pub fn get_all_workitems(&self) -> Result<Vec<PDGWorkItem<'_>>> {
        let context_id = self.get_context_id()?;
        ffi::get_pdg_workitems(&self.node.session, self.node.handle).map(|vec| {
            vec.into_iter()
                .map(|id| PDGWorkItem {
                    id: WorkItemId(id),
                    context_id,
                    node: &self.node,
                })
                .collect()
        })
    }

    pub fn create_workitem(
        &self,
        name: &str,
        index: i32,
        context_id: Option<i32>,
    ) -> Result<PDGWorkItem> {
        let name = std::ffi::CString::new(name)?;
        let context_id = match context_id {
            Some(c) => c,
            None => self.get_context_id()?,
        };
        let id = ffi::create_pdg_workitem(&self.node.session, self.node.handle, &name, index)?;
        Ok(PDGWorkItem {
            id: WorkItemId(id),
            context_id,
            node: &self.node,
        })
    }

    pub fn commit_workitems(&self) -> Result<()> {
        ffi::commit_pdg_workitems(&self.node.session, self.node.handle)
    }
}
