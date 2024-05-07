use std::ffi::OsString;

use bytes::Bytes;
use fuse3::{raw::reply::DirectoryEntry, FileType};
use tokio::io::{AsyncRead, AsyncSeek};

use self::prelude::*;

use super::FuseError;

mod ro;
mod rw;
mod tar;
mod wrapper;

pub const MEMBLOCK_BLOCKSIZE: usize = 4096;

pub mod prelude {
    pub use super::ro::Entry as ReadEntry;
    pub use super::rw::Entry as WriteEntry;
    pub use super::tar::TarTree;
    pub use super::MEMBLOCK_BLOCKSIZE as BLOCKSIZE;
    pub use super::{Entry, InoEntry};
}

#[derive(Debug)]
pub enum Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    Read(ReadEntry<F>),
    Write(WriteEntry),
}

impl<F> Clone for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::Read(arg0) => Self::Read(arg0.clone()),
            Self::Write(arg0) => Self::Write(arg0.clone()),
        }
    }
}

impl<F> Default for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn default() -> Self {
        Self::Write(WriteEntry::Directory)
    }
}

#[derive(Debug)]
pub struct InoEntry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    entry: Entry<F>,
    pub inode: u64,
}

impl<F> InoEntry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    pub async fn kind(&self) -> FileType {
        match &self.entry {
            Entry::Read(read_entry) => read_entry.kind(),
            Entry::Write(write_entry) => write_entry.kind(),
        }
    }
    pub async fn read(&mut self, offset: u64, size: u32) -> Result<Bytes, FuseError> {
        match &mut self.entry {
            Entry::Read(entry) => entry.read(offset, size).await,
            Entry::Write(entry) => entry.read(offset, size).await,
        }
    }
    pub async fn write(&mut self, offset: u64, data: &[u8]) -> Result<u32, FuseError> {
        match &mut self.entry {
            Entry::Read(entry) => entry.write(offset, data).await,
            Entry::Write(entry) => entry.write(offset, data).await,
        }
    }
    // pub async fn
    pub async fn dir_entry(&self, name: OsString) -> DirectoryEntry {
        // But for libfuse, an offset of zero means that offsets are
        // not supported, so we shift everything by one.
        DirectoryEntry {
            inode: self.inode,
            kind: self.kind().await,
            name,
            offset: 0,
        }
    }
}
