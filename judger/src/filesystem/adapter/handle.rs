use std::{collections::BTreeMap, sync::atomic::AtomicU64};

use spin::RwLock;

pub struct HandleTable<E: Clone> {
    handle_generator: AtomicU64,
    table: RwLock<BTreeMap<u64, E>>,
}

impl<E: Clone> HandleTable<E> {
    pub fn new() -> Self {
        Self {
            handle_generator: AtomicU64::new(1),
            table: RwLock::new(BTreeMap::new()),
        }
    }
    pub fn add(&self, entry: E) -> u64 {
        let handle = self
            .handle_generator
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        log::trace!("allocate handle: {}", handle);
        self.table.write().insert(handle, entry);
        handle
    }
    pub fn get(&self, handle: u64) -> Option<E> {
        log::debug!("get handle: {}", handle);
        self.table.read().get(&handle).cloned()
    }
    pub fn remove(&self, handle: u64) -> Option<E> {
        log::trace!("deallocate handle: {}", handle);
        self.table.write().remove(&handle)
    }
}
