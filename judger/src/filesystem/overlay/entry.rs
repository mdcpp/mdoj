use std::{ffi::OsString, sync::Arc};

use tokio::io::{AsyncRead, AsyncSeek};

use crate::{
    filesystem::{tar::Entry, INODE},
    semaphore::*,
};

use super::block::MemBlock;

pub type ArcEntry<F> = Arc<tokio::sync::Mutex<MixedEntry<F>>>;

/// Entry from tar file, it's a replacement of Entry
pub enum MutEntry {
    SymLink(OsString),
    HardLink(INODE),
    Directory,
    File(MemBlock),
    Removed,
}

/// A workaround to not use dynamic dispatch and compact the size of Entry
pub enum MixedEntry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    Mut(Permit, Arc<MutEntry>),
    Immut(Entry<F>),
}

// operation that's per file
// impl MixedEntry {
//     pub async fn read(){}
//     pub async fn write(){}
//     pub async fn flush(){}
//     pub async fn close(){}
// }
