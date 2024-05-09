use std::sync::atomic::{AtomicU64, Ordering};

pub struct Resource(AtomicU64);

impl Resource {
    pub fn new(cap: u64) -> Self {
        Self(AtomicU64::new(cap))
    }
    pub fn comsume(&self, size: u32) -> Option<()> {
        let a = self.0.fetch_sub(size as u64, Ordering::AcqRel);
        if (a | (1 << 63)) != 0 {
            None
        } else {
            Some(())
        }
    }
}
