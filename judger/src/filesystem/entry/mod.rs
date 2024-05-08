use std::{
    ffi::OsString,
    io::SeekFrom,
    sync::atomic::{AtomicI64, Ordering},
};

use bytes::Bytes;
use fuse3::{raw::reply::DirectoryEntry, FileType};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use crate::semaphore::Semaphore;

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

impl<F> ReadEntry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    async fn into_write(
        value: ReadEntry<F>,
        resource: &AtomicI64,
    ) -> Result<WriteEntry, FuseError> {
        if let ReadEntry::File(block) = &value {
            let required_space = block.get_size() as i64;
            if resource.fetch_sub(required_space, Ordering::AcqRel) < required_space {
                return Err(FuseError::OutOfPermit);
            }
        }
        let value = match value {
            ReadEntry::SymLink(target) => WriteEntry::SymLink(target),
            ReadEntry::HardLink(inode) => WriteEntry::HardLink(inode),
            ReadEntry::Directory => WriteEntry::Directory,
            ReadEntry::File(mut block) => {
                block
                    .seek(SeekFrom::Start(0))
                    .await
                    .map_err(|_| FuseError::Underlaying)?;
                let mut data = Vec::new();
                block
                    .read_to_end(&mut data)
                    .await
                    .map_err(|_| FuseError::Underlaying)?;
                WriteEntry::new_data(data)
            }
        };
        Ok(value)
    }
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
    pub async fn read(
        &mut self,
        offset: u64,
        size: u32,
        resource: &AtomicI64,
    ) -> Result<Bytes, FuseError> {
        if let Entry::Read(entry) = &mut self.entry {
            self.entry = Entry::Write(ReadEntry::into_write(entry.clone(), &resource).await?);
        }
        if let Entry::Write(entry) = &mut self.entry {
            entry.read(offset, size).await
        } else {
            unreachable!()
        }
    }
    pub async fn write(
        &mut self,
        offset: u64,
        data: &[u8],
        resource: &AtomicI64,
    ) -> Result<u32, FuseError> {
        let required_space = data.len() as i64;
        if resource.fetch_sub(required_space, Ordering::AcqRel) < required_space {
            return Err(FuseError::OutOfPermit);
        }
        match &mut self.entry {
            Entry::Read(entry) => entry.write(offset, data).await,
            Entry::Write(entry) => entry.write(offset, data).await,
        }
    }
}
