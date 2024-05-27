mod ro;
mod rw;
mod tar;

use self::{ro::TarBlock, rw::MemBlock};
use bytes::Bytes;
use fuse3::FileType;
use std::{ffi::OsString, sync::Arc};
use tokio::{
    io::{AsyncRead, AsyncSeek},
    sync::Mutex,
};

use super::resource::Resource;

pub use tar::TarTree;
pub const BLOCKSIZE: usize = 4096;
const MAX_READ_BLK: usize = 1024;

pub trait FuseReadTrait {
    async fn read(&mut self, offset: u64, size: u32) -> std::io::Result<Bytes>;
}

pub trait FuseWriteTrait {
    async fn write(&mut self, offset: u64, data: &[u8]) -> std::io::Result<u32>;
}

/// Entry in the filesystem
///
/// cloning the entry would clone file state
#[derive(Debug, Default)]
pub enum Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    SymLink(OsString),
    HardLink(u64),
    #[default]
    Directory,
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
    pub fn new_file_with_vec(content: Vec<u8>) -> Self {
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
    pub async fn read(&mut self, offset: u64, size: u32) -> Option<std::io::Result<Bytes>> {
        match self {
            Self::TarFile(block) => Some(Ok(block.read(offset, size).await.unwrap())),
            Self::MemFile(block) => Some(block.read(offset, size).await),
            _ => None,
        }
    }
    pub async fn read_all(&self) -> Option<Vec<u8>> {
        match self {
            Self::TarFile(block) => Some(block.read_all().await.expect("tar ball corrupted")),
            Self::MemFile(block) => None,
            _ => None,
        }
    }
    pub async fn write(
        self_: Arc<Mutex<Self>>,
        offset: u64,
        data: &[u8],
        resource: &Resource,
    ) -> Option<std::io::Result<u32>> {
        let mut lock = self_.lock().await;
        if resource.comsume(data.len() as u32).is_none() {
            return Some(Err(std::io::Error::from(std::io::ErrorKind::Other)));
        }
        match &mut *lock {
            Self::MemFile(block) => Some(block.write(offset, data).await),
            Self::TarFile(block) => Some(Err(std::io::Error::from(std::io::ErrorKind::Other))),
            _ => None,
        }
    }
}
