use std::sync::atomic::{AtomicU64, Ordering};

/// A resource counter
///
/// unlike [`tokio::sync::Semaphore`], the resource is not reusable
pub struct Resource(AtomicU64);

impl Resource {
    /// Create a new resource counter
    pub fn new(cap: u64) -> Self {
        Self(AtomicU64::new(cap))
    }
    /// consume some amount of resource
    pub fn consume(&self, size: u32) -> Option<()> {
        let a = self.0.fetch_sub(size as u64, Ordering::AcqRel);
        if (a & (1 << 63)) != 0 {
            None
        } else {
            Some(())
        }
    }
    /// consume some amount of resource
    ///
    /// return None if the resource is not enough or the size
    /// is out of range (greater than[`u32::MAX`])
    pub fn consume_other<T: TryInto<u32>>(&self, size: T) -> Option<()> {
        let size = size.try_into().ok()?;
        self.consume(size)
    }
}
