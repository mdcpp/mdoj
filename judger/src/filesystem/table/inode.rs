use std::{
    collections::BTreeMap,
    sync::{atomic::AtomicU64, Arc},
};

use spin::RwLock;
use tokio::io::{AsyncRead, AsyncSeek};

use crate::{filesystem::overlay::ArcEntry, semaphore::Semaphore};

pub struct INodeTable<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    semaphore: Arc<Semaphore>,
    table: RwLock<BTreeMap<u64, ArcEntry<F>>>,
    handle_generator: AtomicU64,
}

impl<F> INodeTable<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    pub async fn new_inode(&self, entry: ArcEntry<F>) {
        let handle = self
            .handle_generator
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.table.write().insert(handle, entry);
    }
}
