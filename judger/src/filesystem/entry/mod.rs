mod ro;
mod rw;
mod tar;

use self::{ro::TarBlock, rw::MemBlock};
use bytes::Bytes;
use fuse3::FileType;
use std::{
    ffi::{OsStr, OsString},
    sync::Arc,
};
use tokio::{
    io::{AsyncRead, AsyncSeek},
    sync::Mutex,
};

use super::resource::Resource;

pub use tar::EntryTree;
pub const BLOCKSIZE: usize = 4096;
const MAX_READ_BLK: usize = 1024;

pub trait FuseReadTrait {
    async fn read(&mut self, offset: u64, size: u32) -> std::io::Result<Bytes>;
}

pub trait FuseWriteTrait {
    async fn write(&mut self, offset: u64, data: &[u8]) -> std::io::Result<u32>;
}

pub trait FuseFlushTrait {
    async fn flush(&mut self) -> std::io::Result<()>;
}

/// Entry in the filesystem
///
/// cloning the entry would clone file state
#[derive(Debug, Default)]
pub enum Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    #[default]
    Directory,
    SymLink(OsString),
    HardLink(u64),
    TarFile(TarBlock<F>),
    MemFile(MemBlock),
}

impl<F> Clone for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::SymLink(arg0) => Self::SymLink(arg0.clone()),
            Self::HardLink(arg0) => Self::HardLink(arg0.clone()),
            Self::Directory => Self::Directory,
            Self::TarFile(arg0) => Self::TarFile(arg0.clone()),
            Self::MemFile(arg0) => Self::MemFile(arg0.clone()),
        }
    }
}

impl<F> Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    /// create a new file entry with empty content
    pub fn new_file() -> Self {
        Self::MemFile(MemBlock::default())
    }
    /// create a new file entry with content
    pub fn from_vec(content: Vec<u8>) -> Self {
        Self::MemFile(MemBlock::new(content))
    }
    /// get kind of the file
    pub(super) fn kind(&self) -> FileType {
        match self {
            Self::SymLink(_) => FileType::Symlink,
            Self::HardLink(_) => FileType::RegularFile,
            Self::Directory => FileType::Directory,
            Self::TarFile(_) => FileType::RegularFile,
            Self::MemFile(_) => FileType::RegularFile,
        }
    }
    pub(super) fn get_symlink(&self) -> Option<&OsStr> {
        if let Self::SymLink(x) = self {
            return Some(&*x);
        }
        None
    }
    /// get size of the file
    pub fn get_size(&self) -> u64 {
        match self {
            Self::SymLink(x) => x.len() as u64,
            Self::HardLink(_) => 0,
            Self::Directory => 0,
            Self::TarFile(x) => x.get_size() as u64,
            Self::MemFile(x) => x.get_size(),
        }
    }
    /// pull required bytes from the file
    ///
    /// return Err if the file is not a readable file
    /// or the reading process return io error
    pub async fn read(&mut self, offset: u64, size: u32) -> std::io::Result<Bytes> {
        // FIXME: this implementation is inefficient
        match self {
            Self::TarFile(block) => block.read(offset, size).await,
            Self::MemFile(block) => block.read(offset, size).await,
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "not a file",
            )),
        }
    }
    pub fn assume_tar_file(&self) -> Option<&TarBlock<F>> {
        match self {
            Entry::TarFile(x) => Some(x),
            _ => None,
        }
    }
    pub async fn set_append(&mut self) {
        match self {
            Entry::MemFile(x) => x.set_append(),
            _ => {
                // FIXME: copy on write
            }
        }
    }
    /// write data to the file
    ///
    /// write is garanteed to be successful if the resource is enough and is [`MemFile`]
    pub async fn write(&mut self, offset: u64, data: &[u8], resource: &Resource) -> Option<u32> {
        // FIXME: consume logic should move somewhere else
        let required_size = data.len() as u64 + offset;
        if resource
            .comsume_other(required_size.saturating_sub(self.get_size()))
            .is_none()
        {
            return None;
        }

        match self {
            Self::MemFile(block) => Some(block.write(offset, data).await.unwrap()),
            _ => None,
        }
    }
    pub async fn flush(self_: Arc<Mutex<Self>>) -> Option<std::io::Result<()>> {
        let mut lock = self_.lock().await;
        match &mut *lock {
            Self::MemFile(block) => Some(block.flush().await),
            _ => None,
        }
    }
}
