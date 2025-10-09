use crate::errors::Result;
use crate::node::Session;

#[derive(Debug)]
pub struct AsyncAttribResult<T: Sized + Send + 'static> {
    pub(crate) job_id: i32,
    pub(crate) data: Vec<T>,
    pub(crate) size: usize,
    pub(crate) session: Session,
}

impl<T: Sized + Send + 'static> AsyncAttribResult<T> {
    pub fn is_ready(&self) -> Result<bool> {
        self.session
            .get_job_status(self.job_id)
            .map(|status| status == crate::session::JobStatus::Idle)
    }

    pub fn wait(mut self) -> Result<Vec<T>> {
        loop {
            if self.is_ready()? {
                unsafe {
                    self.data.set_len(self.size);
                }
                return Ok(self.data);
            }
        }
    }
}
