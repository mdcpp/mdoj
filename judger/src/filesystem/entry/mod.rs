use std::{ffi::OsString, ops::Deref, sync::Arc};

use bytes::Bytes;
use fuse3::FileType;
use tokio::{
    io::{AsyncRead, AsyncSeek},
    sync::{Mutex, OwnedMutexGuard},
};

use self::{
    ro::TarBlock,
    rw::MemBlock,
    wrapper::{FuseRead, FuseWrite},
};

use super::{adj::DeepClone, resource::Resource};

mod ro;
mod rw;
mod tar;
mod wrapper;

pub const MEMBLOCK_BLOCKSIZE: usize = 4096;

pub mod prelude {
    pub use super::tar::TarTree;
    pub use super::Entry;
    pub use super::MEMBLOCK_BLOCKSIZE as BLOCKSIZE;
}

async fn clone_arc<T: Clone>(arc: &Arc<Mutex<T>>) -> Arc<Mutex<T>> {
    let inner = arc.deref();
    let lock = inner.lock().await;
    Arc::new(Mutex::new(lock.deref().clone()))
}

#[derive(Debug, Default)]
pub enum Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    SymLink(OsString),
    HardLink(u64),
    #[default]
    Directory,
    TarFile(Arc<Mutex<TarBlock<F>>>),
    MemFile(Arc<Mutex<MemBlock>>),
}

impl<F> DeepClone for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    async fn deep_clone(&self) -> Self {
        match self {
            Self::SymLink(x) => Self::SymLink(x.clone()),
            Self::HardLink(x) => Self::HardLink(*x),
            Self::Directory => Self::Directory,
            Self::TarFile(block) => Self::TarFile(clone_arc(block).await),
            Self::MemFile(block) => Self::MemFile(clone_arc(block).await),
        }
    }
}

impl<F> Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    pub fn kind(&self) -> FileType {
        match self {
            Self::SymLink(_) => FileType::Symlink,
            Self::HardLink(_) => FileType::RegularFile,
            Self::Directory => FileType::Directory,
            Self::TarFile(_) => FileType::RegularFile,
            Self::MemFile(_) => FileType::RegularFile,
        }
    }
    pub fn get_size(&self) -> u64 {
        match self {
            Self::SymLink(x) => x.len() as u64,
            Self::HardLink(_) => 0,
            Self::Directory => 0,
            Self::TarFile(_) | Self::MemFile(_) => 1,
        }
    }
    pub fn get_read_handle(&self) -> Option<ReadHandle<F>> {
        match self {
            Self::TarFile(block) => Some(ReadHandle::TarFile(block.clone())),
            Self::MemFile(block) => Some(ReadHandle::MemFile(block.clone())),
            _ => None,
        }
    }
    pub fn get_write_handle(&mut self) -> Option<WriteHandle<F>> {
        match self {
            Self::TarFile(block) => {
                let tar_block = block.clone();
                let mem_block = Arc::new(Mutex::new(MemBlock::new(Vec::new())));
                *self = Self::MemFile(mem_block.clone());
                Some(WriteHandle::TarFile(
                    tar_block,
                    mem_block.try_lock_owned().unwrap(),
                ))
            }
            Self::MemFile(block) => Some(WriteHandle::MemFile(block.clone())),
            _ => None,
        }
    }
}

pub enum ReadHandle<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    TarFile(Arc<Mutex<TarBlock<F>>>),
    MemFile(Arc<Mutex<MemBlock>>),
}

impl<F> ReadHandle<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    pub async fn read(&self, offset: u64, size: u32) -> std::io::Result<Bytes> {
        match self {
            Self::TarFile(block) => {
                let mut lock = block.lock().await;
                FuseRead(&mut *lock).read(offset, size).await
            }
            Self::MemFile(block) => {
                let mut lock = block.lock().await;
                FuseRead(&mut *lock).read(offset, size).await
            }
        }
    }
}

pub enum WriteHandle<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    TarFile(Arc<Mutex<TarBlock<F>>>, OwnedMutexGuard<MemBlock>),
    MemFile(Arc<Mutex<MemBlock>>),
}

impl<F> WriteHandle<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    pub async fn write(
        &self,
        offset: u64,
        data: &[u8],
        resource: &Resource,
    ) -> std::io::Result<u32> {
        match self {
            Self::TarFile(tar_block, mem_block) => {
                // let mut lock = tar_block.lock().await;
                // let mem_block=MemBlock::new(lock.read_all().await.unwrap());
                todo!()
            }
            Self::MemFile(block) => {
                resource
                    .comsume(data.len() as u32)
                    .ok_or(std::io::Error::from(std::io::ErrorKind::Other))?;
                let mut lock = block.lock().await;
                FuseWrite(&mut *lock).write(offset, data).await
            }
        }
    }
}
