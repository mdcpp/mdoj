// FIXME: this module is not well designed, it didn't implement meaningful logic(kind like adapter?)
pub mod handle;
pub(self) mod semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::error::Error;

const MAX_WAIT: usize = 10;
static CGROUP_PREFIX: &str = "mdoj.";

pub struct Daemon {
    cg_name_counter: AtomicUsize,
    memory: semaphore::Semaphore,
}

impl Daemon {
    pub fn new(memory: u64) -> Self {
        Self {
            cg_name_counter: AtomicUsize::new(0),
            memory: semaphore::Semaphore::new(memory, MAX_WAIT),
        }
    }
    // FIXME: daemon should provide direct interface for sandbox, which should rely on process
    pub(super) async fn spawn_handle(&self, resource: u64) -> Result<handle::Handle, Error> {
        let cg_name = format!(
            "{}{}",
            CGROUP_PREFIX,
            self.cg_name_counter.fetch_add(1, Ordering::AcqRel)
        );
        let memory = self.memory.get_permit(resource).await?;
        Ok(handle::Handle::new(cg_name, memory))
    }
}
