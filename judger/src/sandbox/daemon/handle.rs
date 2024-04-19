use super::semaphore::*;

pub struct Handle {
    cg_name: String,
    memory: Permit,
}

impl Handle {
    pub(super) fn new(cg_name: String, memory: Permit) -> Self {
        Self { cg_name, memory }
    }
    pub fn get_cg_name(&self) -> &str {
        &self.cg_name
    }
}
