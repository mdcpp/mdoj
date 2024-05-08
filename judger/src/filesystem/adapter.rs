use std::{ffi::OsStr, num::NonZeroU32, path::Path, sync::atomic::AtomicI64, time::Duration};

use crate::{filesystem::FuseError, semaphore::Permit, Error};
use bytes::Bytes;
use fuse3::{
    raw::{reply::*, MountHandle, Request, Session},
    FileType, MountOptions, Result as FuseResult,
};
use std::future::{ready as future_ready, Future};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek},
};

use super::{entry::prelude::*, reply::*, table::HandleTable, tree::ArcNode};

type VecStream<I> = tokio_stream::Iter<std::vec::IntoIter<I>>;

pub struct FilesystemHandle(Option<MountHandle>);

impl Drop for FilesystemHandle {
    fn drop(&mut self) {
        tokio::spawn(self.0.take().unwrap().unmount());
    }
}

pub struct Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    handle_table: HandleTable<ArcNode<InoEntry<F>>>,
    tree: TarTree<F>,
    resource: AtomicI64,
    _permit: Permit,
}

impl<F> Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
{
    pub async fn mount(self, path: impl AsRef<Path> + Clone) -> FilesystemHandle {
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };

        let mut mount_options = MountOptions::default();

        mount_options.uid(uid).gid(gid).force_readdir_plus(true);

        let handle = Session::new(mount_options)
            .mount_with_unprivileged(self, path.as_ref())
            .await
            .unwrap();

        FilesystemHandle(Some(handle))
    }
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
                    return Ok(reply_entry(req, entry).await);
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
            let fh = self.handle_table.add(entry);
            Ok(ReplyOpen { fh, flags: 0 })
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
                .read(offset, size, &self.resource)
                .await
                .map(|data| ReplyData { data })
                .map_err(Into::into)
        }
    }
    // fn readdir<'a>(
    //     &'a self,
    //     req: Request,
    //     parent: u64,
    //     fh: u64,
    //     offset: i64,
    // ) -> impl Future<Output = FuseResult<ReplyDirectory<Self::DirEntryStream<'a>>>> + Send {
    //     async move {
    //         let entry = self.tree.inode.get(parent).ok_or(FuseError::NotDir)?;
    //         let entry = entry.read().await;
    //         // FIXME: use stream rather than vec iterator
    //         let mut result: Vec<Result<DirectoryEntry, _>> = Vec::new();
    //         for (name, child) in entry.list_child() {
    //             result.push(Ok(dir_entry(name.to_os_string(), child).await));
    //         }
    //         Ok(ReplyDirectory {
    //             entries: tokio_stream::iter(result.into_iter()),
    //         })
    //     }
    // }
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
        // FIXME: use semaphore to limit the write
        async move {
            let entry = self.handle_table.get(fh).ok_or(FuseError::HandleNotFound)?;
            let mut entry = entry.write().await;
            entry
                .write(offset, data, &self.resource)
                .await
                .map(|written| ReplyWrite { written })
                .map_err(Into::into)
        }
    }
    fn open(
        &self,
        req: Request,
        inode: u64,
        flags: u32,
    ) -> impl Future<Output = FuseResult<ReplyOpen>> + Send {
        async move {
            let entry = self.tree.inode.get(inode).ok_or(FuseError::InodeNotFound)?;
            let fh = self.handle_table.add(entry);
            Ok(ReplyOpen { fh, flags: 0 })
        }
    }
    fn readdirplus<'a>(
        &'a self,
        req: Request,
        parent: u64,
        fh: u64,
        offset: u64,
        lock_owner: u64,
    ) -> impl Future<Output = FuseResult<ReplyDirectoryPlus<Self::DirEntryPlusStream<'a>>>> + Send
    {
        async move {
            let entry = self.tree.inode.get(parent).ok_or(FuseError::NotDir)?;
            let entry = entry.read().await;
            if entry.kind().await != FileType::Directory {
                return Err(FuseError::NotDir.into());
            }
            // FIXME: use stream rather than vec iterator
            let parent_attr = file_attr(&entry).await;
            let mut result: Vec<Result<DirectoryEntryPlus, _>> = Vec::new();
            log::info!("parent inode: {}", parent);
            for (name, child) in entry.list_child().skip(offset as usize) {
                let a = dir_entry_plus(parent_attr, name.to_os_string(), child).await;
                log::info!("child inode: {}", a.inode);
                result.push(Ok(a));
            }

            Ok(ReplyDirectoryPlus {
                entries: tokio_stream::iter(result.into_iter()),
            })
        }
    }
    fn fallocate(
        &self,
        req: Request,
        inode: u64,
        fh: u64,
        offset: u64,
        length: u64,
        mode: u32,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        async move {
            if let Some(entry) = self.tree.inode.get(inode) {
                match entry.read().await.kind().await {
                    FileType::Directory | FileType::NamedPipe | FileType::CharDevice => {}
                    _ => return Ok(()),
                }
            }
            Err(FuseError::IsDir.into())
        }
    }

    fn interrupt(&self, req: Request, unique: u64) -> impl Future<Output = FuseResult<()>> + Send {
        future_ready(Ok(()))
    }
    fn getattr(
        &self,
        req: Request,
        inode: u64,
        fh: Option<u64>,
        flags: u32,
    ) -> impl Future<Output = FuseResult<ReplyAttr>> + Send {
        async move {
            let root = self.tree.tree.get_root();
            let entry = root.read().await;
            Ok(ReplyAttr {
                ttl: Duration::from_secs(30),
                attr: file_attr(&entry).await,
            })
        }
    }
    // fn create(
    //     &self,
    //     req: Request,
    //     parent: u64,
    //     name: &OsStr,
    //     mode: u32,
    //     flags: u32,
    // ) -> impl core::future::Future<Output = FuseResult<ReplyCreated>> + Send {
    //     async move {
    //         let parent = self
    //             .tree
    //             .inode
    //             .get(parent)
    //             .ok_or(FuseError::InodeNotFound)?;
    //         let mut parent = parent.write().await;
    //         let child = parent.create(name, mode).await?;
    //         let fh = self.handle_table.add(child);
    //         Ok(ReplyCreated {
    //             entry: child.dir_entry(name.to_os_string()).await,
    //             ttl: 0,
    //             flags: 0,
    //             fh,
    //         })
    //     }
    // }
}

pub struct Template<F>(TarTree<F>)
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static;

impl<F> Template<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn new_inner(tree: TarTree<F>) -> Self {
        Self(tree)
    }
    pub async fn as_filesystem(&self, permit: Permit) -> Filesystem<F> {
        Filesystem {
            handle_table: HandleTable::new(),
            tree: self.0.cloned().await,
            resource: AtomicI64::new(
                permit
                    .count()
                    .try_into()
                    .expect(&format!("filesystem max size: {}", i64::MAX - 1)),
            ),
            _permit: permit,
        }
    }
}

impl Template<File> {
    pub async fn new(path: impl AsRef<Path> + Clone) -> Result<Self, Error> {
        let tree = TarTree::new(path).await?;
        Ok(Self::new_inner(tree))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::semaphore::Semaphore;
    use env_logger::*;
    use tokio::time;

    #[tokio::test]
    #[ignore = "not meant to be tested"]
    async fn real_run() {
        Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .try_init()
            .ok();

        log::info!("start");
        let global_resource = Semaphore::new(4096 * 1024 * 1024, 1);
        let template = Template::new("test/nested.tar").await.unwrap();
        let filesystem = template
            .as_filesystem(
                global_resource
                    .get_permit(1024 * 1024 * 1024)
                    .await
                    .unwrap(),
            )
            .await;
        let handle = filesystem.mount("./.temp/21").await;

        time::sleep(time::Duration::from_secs(300)).await;
        drop(handle);
    }
}
