//! collection of entry
//!
//! In tar file, structure is like this:
//! | type | content | ...
//!
//! And we map each type of content to BTreeMap<PathBuf, Entry>

use crate::semaphore::*;
use std::{ffi::OsString, sync::Arc};

use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek},
};

use super::block::TarBlock;

/// Entry from tar file, should be readonly
#[derive(Debug)]
pub enum Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    Link(Arc<OsString>),
    Directory,
    File(TarBlock<F>),
}

impl<F> PartialEq for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Link(l0), Self::Link(r0)) => l0 == r0,
            (Self::File(l0), Self::File(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

/// Entry from tar file, it's a replacement of Entry
pub enum MutEntry {
    Link(OsString),
    Directory,
    File(Vec<u8>),
    Removed,
}

/// A workaround to not use dynamic dispatch and compact the size of Entry
pub enum MixedEntry {
    Mut(Permit, Arc<MutEntry>),
    Immut(Entry<File>),
}
