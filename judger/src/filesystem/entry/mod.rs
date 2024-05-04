use fuse3::FileType;
use tokio::{
    io::{AsyncRead, AsyncSeek},
    sync::RwLock,
};

use self::prelude::*;

mod ro;
mod rw;
mod template;

pub const MEMBLOCK_BLOCKSIZE: usize = 4096;

mod prelude {
    pub use super::ro::Entry as ReadEntry;
    pub use super::rw::Entry as WriteEntry;
    pub use super::MEMBLOCK_BLOCKSIZE;
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
}
