use std::{
    collections::BTreeMap,
    mem::ManuallyDrop,
    sync::atomic::{AtomicU64, Ordering},
};

use futures_core::{future::BoxFuture, Future};
use spin::RwLock;

const MAX_INODE: u64 = 1 << 63;

pub struct InodeHandle<'a, E: Clone> {
    inode: u64,
    table: &'a InodeTable<E>,
}

impl<'a, E: Clone> Drop for InodeHandle<'a, E> {
    fn drop(&mut self) {
        panic!("InodeHandle should be consumed by allocate")
    }
}

impl<'a, E: Clone> InodeHandle<'a, E> {
    pub fn get_inode(&self) -> u64 {
        self.inode
    }
    pub fn consume(self, value: E) {
        assert!(self.table.inode.write().insert(self.inode, value).is_none());
        ManuallyDrop::new(self);
    }
}

pub struct InodeTable<E: Clone> {
    inode: RwLock<BTreeMap<u64, E>>,
    inode_generator: AtomicU64,
}

impl<E: Clone> Default for InodeTable<E> {
    fn default() -> Self {
        Self {
            inode: RwLock::new(BTreeMap::new()),
            inode_generator: AtomicU64::new(2),
        }
    }
}

impl<E: Clone> InodeTable<E> {
    /// clone the inode table
    ///
    /// Note: it only deep clone geberator and inode, not the entry
    pub fn cloned(&self) -> Self {
        Self {
            inode: RwLock::new(self.inode.read().clone()),
            inode_generator: AtomicU64::new(self.inode_generator.load(Ordering::SeqCst)),
        }
    }
    /// allocate root inode with handle
    pub fn allocate_root(&self) -> InodeHandle<E> {
        InodeHandle {
            inode: 1,
            table: self,
        }
    }
    /// allocate inode with handle
    pub fn allocate(&self) -> InodeHandle<E> {
        let inode = self
            .inode_generator
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        InodeHandle { inode, table: self }
    }
    /// update(clone) entry by providing new entry with inode
    pub async fn clone_update_entry(&self, inode: u64, entry: E) {
        self.inode.write().insert(inode, entry);
    }
    /// get entry by inode
    pub fn get(&self, inode: u64) -> Option<E> {
        self.inode.read().get(&inode).cloned()
    }
    /// deallocate inode
    pub fn remove(&self, inode: u64) {
        // FIXME: inode should be reused, currently it's a non-op
    }
    pub fn get_free_inode(&self) -> u64 {
        MAX_INODE - self.get_used_inode()
    }
    #[inline]
    pub fn get_used_inode(&self) -> u64 {
        self.inode_generator.load(Ordering::Acquire)
    }
}
