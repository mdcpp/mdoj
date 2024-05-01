use std::{collections::BTreeMap, sync::atomic::AtomicU64};

use spin::RwLock;
use tokio::io::{AsyncRead, AsyncSeek};

use crate::filesystem::overlay::*;

pub struct HandleTable<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    handle_generator: AtomicU64,
    table: RwLock<BTreeMap<u64, ArcEntry<F>>>,
}

impl<F> HandleTable<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    pub fn new_handle(&self, entry: ArcEntry<F>) -> u64 {
        let handle = self
            .handle_generator
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.table.write().insert(handle, entry);
        handle
    }
    pub fn get_entry(&self, handle: u64) -> Option<ArcEntry<F>> {
        self.table.read().get(&handle).cloned()
    }
    pub fn remove_entry(&self, handle: u64) -> Option<ArcEntry<F>> {
        self.table.write().remove(&handle)
    }
}
