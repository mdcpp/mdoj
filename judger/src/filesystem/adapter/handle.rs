use std::{
    collections::BTreeMap,
    sync::{atomic::AtomicU64, Arc},
};

use spin::Mutex;

pub struct HandleTable<E> {
    handle_generator: AtomicU64,
    table: Mutex<BTreeMap<u64, Arc<E>>>,
}

impl<E> HandleTable<E> {
    /// Create a new handle table
    pub fn new() -> Self {
        Self {
            handle_generator: AtomicU64::new(1),
            table: Mutex::new(BTreeMap::new()),
        }
    }
    /// Add an entry to the table
    pub fn add(&self, entry: E) -> u64 {
        let handle = self
            .handle_generator
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        log::trace!("allocate handle: {}", handle);
        self.table.lock().insert(handle, Arc::new(entry));
        handle
    }
    /// Get an entry from the table
    pub fn get(&self, handle: u64) -> Option<Arc<E>> {
        log::debug!("get handle: {}", handle);
        self.table.lock().get(&handle).cloned()
    }
    /// Remove an entry from the table
    pub fn remove(&self, handle: u64) -> Option<Arc<E>> {
        log::trace!("deallocate handle: {}", handle);
        self.table.lock().remove(&handle)
    }
}
