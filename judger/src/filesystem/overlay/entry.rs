use std::{
    ffi::{OsStr, OsString},
    sync::Arc,
};

use tokio::io::{AsyncRead, AsyncSeek};

use crate::filesystem::{
    table::{INodeTable, Identified},
    tar::Entry as RoEntry,
    tree::ArcNode,
};

use super::block::MemBlock;

type ArcEntry<F> = ArcNode<EntryKind<F>>;

/// Entry from tar file, it's a replacement of Entry
#[derive(Default)]
pub enum RwEntry {
    SymLink(OsString),
    HardLink(u64),
    #[default]
    Directory,
    File(MemBlock),
    Removed,
}

/// A workaround to not use dynamic dispatch and compact the size of Entry
pub enum EntryKind<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    Rw(ArcNode<RwEntry>),
    Ro(ArcNode<RoEntry<F>>),
}

impl<F> Clone for EntryKind<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::Rw(arg0) => Self::Rw(arg0.clone()),
            Self::Ro(arg0) => Self::Ro(arg0.clone()),
        }
    }
}

impl<F> EntryKind<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    pub async fn get_child_by_componment(&self, component: &OsStr) -> Option<Self> {
        match self {
            Self::Rw(node) => (node.read().await.get_by_component(component)).map(Self::Rw),
            Self::Ro(node) => todo!(),
        }
    }
}

pub struct Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    pub kind: EntryKind<F>,
    inode: u64,
}

impl<F> Identified for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn get_id(&self) -> usize {
        match &self.kind {
            EntryKind::Rw(x) => Arc::as_ptr(x) as usize,
            EntryKind::Ro(x) => Arc::as_ptr(x) as usize,
        }
    }
}

impl<F> Clone for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn clone(&self) -> Self {
        Self {
            kind: self.kind.clone(),
            inode: self.inode.clone(),
        }
    }
}

impl<F> Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    #[inline]
    pub fn from_kind(kind: EntryKind<F>, inode: u64) -> Self {
        Self { kind, inode }
    }
    #[inline]
    pub fn get_inode(&self) -> u64 {
        self.inode
    }
}

impl<F> INodeTable<Entry<F>>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    pub fn add_entry_rw(&self, entry: ArcNode<RwEntry>) -> Entry<F> {
        self.allocate(|x| Entry::from_kind(EntryKind::Rw(entry.clone()), x))
    }
    pub fn add_entry_ro(&self, entry: ArcNode<RoEntry<F>>) -> Entry<F> {
        self.allocate(|x| Entry::from_kind(EntryKind::Ro(entry.clone()), x))
    }
    pub fn add_entry_kind(&self, kind: EntryKind<F>) -> Entry<F> {
        self.allocate(|x| Entry::from_kind(kind.clone(), x))
    }
    pub fn lookup(&self, inode: u64) -> Option<Entry<F>> {
        self.get(inode)
    }
    pub async fn get_child_by_componment(&self, inode: u64, component: &OsStr) -> Option<Entry<F>> {
        if let Some(entry) = self.get(inode) {
            return entry
                .kind
                .get_child_by_componment(component)
                .await
                .map(|kind| self.add_entry_kind(kind));
        }
        None
    }
}
