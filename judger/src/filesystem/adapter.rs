use std::{ffi::OsStr, num::NonZeroU32, path::Path};

use crate::{
    filesystem::{reply::ImmutParsable, FuseError},
    semaphore::{Permit, Semaphore},
    Error,
};
use fuse3::{
    raw::{reply::*, Request},
    FileType, Result as FuseResult,
};
use std::future::{ready as future_ready, Future};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek},
};

use super::{entry::prelude::*, table::HandleTable, tree::ArcNode};

type VecStream<I> = tokio_stream::Iter<std::vec::IntoIter<I>>;

pub struct Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    handle_table: HandleTable<ArcNode<InoEntry<F>>>,
    tree: TarTree<F>,
    semaphore: Semaphore,
    _permit: Permit,
}

impl<F> fuse3::raw::Filesystem for Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
{
    type DirEntryStream<'a>=VecStream<FuseResult<DirectoryEntry>> where Self: 'a;
    type DirEntryPlusStream<'a>=VecStream<FuseResult<DirectoryEntryPlus>> where Self: 'a;

    fn init(&self, _: Request) -> impl Future<Output = FuseResult<ReplyInit>> + Send {
        future_ready(Ok(ReplyInit {
            max_write: NonZeroU32::new(BLOCKSIZE as u32).unwrap(),
        }))
    }

    fn destroy(&self, _: Request) -> impl Future<Output = ()> + Send {
        future_ready(())
    }

    fn lookup(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
    ) -> impl Future<Output = FuseResult<ReplyEntry>> + Send {
        async move {
            if let Some(entry) = self.tree.inode.get(parent) {
                let entry = entry.read().await;
                if let Some(entry) = entry.get_by_component(name) {
                    return Ok(ReplyEntry::parse(req, entry).await);
                }
            }
            Err(FuseError::InodeNotFound.into())
        }
    }
    fn forget(
        &self,
        _: Request,
        inode: u64,
        _: u64,
    ) -> impl core::future::Future<Output = ()> + Send {
        self.tree.inode.remove(inode);
        future_ready(())
    }
    fn statfs(
        &self,
        _: Request,
        inode: u64,
    ) -> impl Future<Output = FuseResult<ReplyStatFs>> + Send {
        async {
            Ok(ReplyStatFs {
                blocks: 0,
                bfree: 4096 * 4096,
                bavail: 4096 * 2048,
                files: 0,
                ffree: self.tree.inode.get_free_inode(),
                bsize: BLOCKSIZE as u32,
                namelen: 256,
                frsize: BLOCKSIZE as u32,
            })
        }
    }
    fn opendir(
        &self,
        req: Request,
        inode: u64,
        flags: u32,
    ) -> impl Future<Output = FuseResult<ReplyOpen>> + Send {
        async move {
            let entry = self.tree.inode.get(inode).ok_or(FuseError::InodeNotFound)?;
            if entry.read().await.kind().await != FileType::Directory {
                return Err(FuseError::NotDir.into());
            }
            let handle = self.handle_table.add(entry);
            Ok(ReplyOpen {
                fh: handle,
                flags: 0,
            })
        }
    }
    fn read(
        &self,
        req: Request,
        inode: u64,
        fh: u64,
        offset: u64,
        size: u32,
    ) -> impl Future<Output = FuseResult<ReplyData>> + Send {
        async move {
            let entry = self.handle_table.get(fh).ok_or(FuseError::HandleNotFound)?;
            let mut entry = entry.write().await;
            entry
                .read(offset, size)
                .await
                .map(|data| ReplyData { data })
                .map_err(Into::into)
        }
    }
    fn readdir<'a>(
        &'a self,
        req: Request,
        parent: u64,
        fh: u64,
        offset: i64,
    ) -> impl Future<Output = FuseResult<ReplyDirectory<Self::DirEntryStream<'a>>>> + Send {
        async move {
            let entry = self.handle_table.get(fh).ok_or(FuseError::NotDir)?;
            let entry = entry.read().await;
            // FIXME: use stream rather than vec iterator
            let mut result: Vec<Result<DirectoryEntry, _>> = Vec::new();
            for (name, child) in entry.list_child() {
                let child = child.read().await;
                result.push(Ok(child.dir_entry(name.to_os_string()).await));
            }
            Ok(ReplyDirectory {
                entries: tokio_stream::iter(result.into_iter()),
            })
        }
    }
    fn access(
        &self,
        req: Request,
        inode: u64,
        mask: u32,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        future_ready(Ok(()))
    }
    fn fsync(
        &self,
        req: Request,
        inode: u64,
        fh: u64,
        datasync: bool,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        future_ready(Ok(()))
    }
    fn fsyncdir(
        &self,
        req: Request,
        inode: u64,
        fh: u64,
        datasync: bool,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        future_ready(Ok(()))
    }
    fn write(
        &self,
        req: Request,
        inode: u64,
        fh: u64,
        offset: u64,
        data: &[u8],
        write_flags: u32,
        flags: u32,
    ) -> impl Future<Output = FuseResult<ReplyWrite>> + Send {
        /// FIXME: use semaphore to limit the write
        async move {
            let entry = self.handle_table.get(fh).ok_or(FuseError::HandleNotFound)?;
            let mut entry = entry.write().await;
            entry
                .write(offset, data)
                .await
                .map(|written| ReplyWrite { written })
                .map_err(Into::into)
        }
    }
}

pub struct Template<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    tree: TarTree<F>,
    semaphore: Semaphore,
}

impl<F> Template<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn new_inner(tree: TarTree<F>, memory: u64) -> Self {
        Self {
            tree,
            semaphore: Semaphore::new(memory, 10),
        }
    }
    pub async fn as_template(&self, size: u64) -> Filesystem<F> {
        Filesystem {
            handle_table: HandleTable::new(),
            tree: self.tree.cloned().await,
            semaphore: self.semaphore.clone(),
            _permit: self.semaphore.get_permit(size).await.unwrap(),
        }
    }
}

impl Template<File> {
    pub async fn new(path: impl AsRef<Path> + Clone, size: u64) -> Result<Self, Error> {
        let tree = TarTree::new(path).await?;
        Ok(Self::new_inner(tree, size))
    }
}
