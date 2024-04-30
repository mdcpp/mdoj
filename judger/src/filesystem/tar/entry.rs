//! collection of entry
//!
//! In tar file, structure is like this:
//! | type | content | ...
//!
//! And we map each type of content to BTreeMap<PathBuf, Entry>

use std::{ffi::OsString, sync::Arc};

use tokio::io::{AsyncRead, AsyncSeek};

use crate::filesystem::INODE;

use super::block::TarBlock;

/// Entry from tar file, should be readonly
#[derive(Debug)]
pub enum Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    SymLink(Arc<OsString>),
    HardLink(INODE),
    Directory,
    File(TarBlock<F>),
}

// impl<F> Entry<F>
// where
//     F: AsyncRead + AsyncSeek + Unpin + 'static,
// {
//     pub fn read(&mut self)
// }

impl<F> PartialEq for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SymLink(l0), Self::SymLink(r0)) => l0 == r0,
            (Self::File(l0), Self::File(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
