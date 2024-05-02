use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};

use spin::RwLock;

const MAX_INODE: u64 = 1 << 63;

pub trait Identified {
    fn get_id(&self) -> usize;
}

pub struct INodeTable<E: Clone + Identified> {
    inode: RwLock<BTreeMap<u64, E>>,
    id: RwLock<BTreeMap<usize, u64>>,
    inode_generator: AtomicU64,
}

impl<E: Clone + Identified> INodeTable<E> {
    pub fn new() -> Self {
        Self {
            inode: RwLock::new(BTreeMap::new()),
            id: RwLock::new(BTreeMap::new()),
            inode_generator: AtomicU64::new(1),
        }
    }
    pub fn allocate<F>(&self, mut f: F) -> E
    where
        F: FnMut(u64) -> E,
    {
        let inode = self
            .inode_generator
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let entry = f(inode);
        match { self.id.read().get(&entry.get_id()) } {
            Some(&x) => f(x),
            None => {
                self.inode.write().insert(inode, entry.clone());
                entry
            }
        }
    }
    /// get entry by inode
    pub fn get(&self, inode: u64) -> Option<E> {
        self.inode.read().get(&inode).cloned()
    }
    /// deallocate inode
    pub fn remove(&self, inode: u64) {
        // FIXME: inode should be reused
        if let Some(e) = { self.inode.write().remove(&inode) } {
            self.id.write().remove(&e.get_id());
        }
    }
    pub fn get_free_inode(&self) -> u64 {
        MAX_INODE - self.get_used_inode()
    }
    #[inline]
    pub fn get_used_inode(&self) -> u64 {
        self.inode_generator.load(Ordering::Acquire)
    }
}
