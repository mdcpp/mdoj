use std::{ffi::OsStr, num::NonZeroU32, path::Path, sync::Arc};

use futures_core::Future;
use spin::Mutex;
use tokio::io::{AsyncRead, AsyncSeek};

use crate::{
    filesystem::{resource::Resource, TarTree, BLOCKSIZE},
    semaphore::Permit,
};

use super::{error::FuseError, handle::HandleTable, reply::*};
use fuse3::{
    raw::{reply::*, *},
    Result as FuseResult, *,
};

type VecStream<I> = tokio_stream::Iter<std::vec::IntoIter<I>>;
pub struct Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    handle_table: HandleTable<usize>,
    tree: Mutex<TarTree<F>>,
    resource: Arc<Resource>,
    _permit: Permit,
}

impl<F> Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
{
    pub(super) fn new(tree: TarTree<F>, permit: Permit) -> Self {
        Self {
            handle_table: HandleTable::new(),
            tree: Mutex::new(tree),
            resource: Arc::new(Resource::new(permit.count())),
            _permit: permit,
        }
    }
    pub async fn mount(self, path: impl AsRef<Path> + Clone) -> std::io::Result<MountHandle> {
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };

        let mut mount_options = MountOptions::default();

        mount_options.uid(uid).gid(gid);

        Session::new(mount_options)
            .mount_with_unprivileged(self, path.as_ref())
            .await
    }
}

impl<F> fuse3::raw::Filesystem for Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
{
    type DirEntryStream<'a>=VecStream<FuseResult<DirectoryEntry>> where Self: 'a;
    type DirEntryPlusStream<'a>=VecStream<FuseResult<DirectoryEntryPlus>> where Self: 'a;

    fn init(&self, _: Request) -> impl Future<Output = FuseResult<ReplyInit>> + Send {
        async {
            Ok(ReplyInit {
                max_write: NonZeroU32::new(BLOCKSIZE as u32).unwrap(),
            })
        }
    }

    fn destroy(&self, _: Request) -> impl Future<Output = ()> + Send {
        async {}
    }

    fn lookup(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
    ) -> impl Future<Output = FuseResult<ReplyEntry>> + Send {
        async move {
            let tree = self.tree.lock();
            let node = tree.get(parent as usize).ok_or(FuseError::InvaildIno)?;
            log::info!(
                "parent name: {}",
                String::from_utf8_lossy(node.get_name().as_encoded_bytes())
            );
            log::info!(
                "lookup name: {}",
                String::from_utf8_lossy(name.as_encoded_bytes())
            );
            let entry = node.get_by_component(name).ok_or(FuseError::InvalidPath)?;
            // FIXME: unsure about the inode
            Ok(reply_entry(req, entry.get_value(), parent))
        }
    }
    fn forget(&self, _: Request, inode: u64, _: u64) -> impl Future<Output = ()> + Send {
        async {}
    }
    fn statfs(
        &self,
        _: Request,
        inode: u64,
    ) -> impl Future<Output = FuseResult<ReplyStatFs>> + Send {
        async {
            let tree = self.tree.lock();
            Ok(ReplyStatFs {
                blocks: 0,
                bfree: 4096 * 4096,
                bavail: 4096 * 2048,
                files: 0,
                ffree: tree.get_remain_capacity() as u64,
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
            let tree = self.tree.lock();
            let node = tree.get(inode as usize).ok_or(FuseError::InvaildIno)?;
            if node.get_value().kind() != FileType::Directory {
                return Err(FuseError::NotDir.into());
            }
            let fh = self.handle_table.add(node.get_id());
            Ok(ReplyOpen { fh, flags: 0 })
        }
    }
    fn open(
        &self,
        req: Request,
        inode: u64,
        flags: u32,
    ) -> impl Future<Output = FuseResult<ReplyOpen>> + Send {
        async move {
            let tree = self.tree.lock();
            let entry = tree.get(inode as usize).ok_or(FuseError::InvaildIno)?;
            let fh = self.handle_table.add(entry.get_id());
            Ok(ReplyOpen { fh, flags: 0 })
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
            let tree = self.tree.lock();
            let node = tree.get(parent as usize).ok_or(FuseError::InvaildIno)?;

            if node.get_value().kind() != FileType::Directory {
                return Err(FuseError::NotDir.into());
            }

            let parent_node = node.parent().unwrap_or_else(|| tree.get_root());

            let entries = vec![
                Ok(dir_entry(
                    OsStr::new(".").to_os_string(),
                    node.get_value(),
                    node.get_id() as u64,
                )),
                Ok(dir_entry(
                    OsStr::new("..").to_os_string(),
                    parent_node.get_value(),
                    parent_node.get_id() as u64,
                )),
            ]
            .into_iter()
            .chain(
                node.children()
                    .map(|inode| {
                        let node = tree.get(inode).unwrap();
                        dir_entry(
                            node.get_name().to_os_string(),
                            node.get_value(),
                            inode as u64,
                        )
                    })
                    .map(Ok),
            )
            .skip(offset as usize)
            .collect::<Vec<_>>();

            Ok(ReplyDirectory {
                entries: tokio_stream::iter(entries),
            })
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
            let tree = self.tree.lock();
            let node = tree.get(parent as usize).ok_or(FuseError::InvaildIno)?;

            if node.get_value().kind() != FileType::Directory {
                return Err(FuseError::NotDir.into());
            }

            let parent_node = node.parent().unwrap_or_else(|| tree.get_root());

            let entries = vec![
                Ok(dir_entry_plus(
                    OsStr::new(".").to_os_string(),
                    node.get_value(),
                    node.get_id() as u64,
                    1,
                )),
                Ok(dir_entry_plus(
                    OsStr::new("..").to_os_string(),
                    parent_node.get_value(),
                    parent_node.get_id() as u64,
                    2,
                )),
            ]
            .into_iter()
            .chain(
                node.children()
                    .enumerate()
                    .map(|(offset, inode)| {
                        let node = tree.get(inode).unwrap();
                        dir_entry_plus(
                            node.get_name().to_os_string(),
                            node.get_value(),
                            inode as u64,
                            (offset + 3) as i64,
                        )
                    })
                    .map(Ok),
            )
            .skip(offset as usize)
            .collect::<Vec<_>>();

            Ok(ReplyDirectoryPlus {
                entries: tokio_stream::iter(entries),
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
            let handle = {
                let tree = self.tree.lock();
                let entry = self.handle_table.get(fh).ok_or(FuseError::HandleNotFound)?;
                let node = tree.get(entry).ok_or(FuseError::InvaildIno)?;
                let entry = node.get_value();
                entry.get_read_handle()
            }
            .ok_or(FuseError::IsDir)?;

            handle
                .read(offset, size)
                .await
                .map(|data| ReplyData { data })
                .map_err(Into::into)
        }
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
        async move {
            let handle = {
                let mut tree = self.tree.lock();
                let entry = self.handle_table.get(fh).ok_or(FuseError::HandleNotFound)?;
                let mut node = tree.get_mut(entry).ok_or(FuseError::InvaildIno)?;
                let entry = node.get_value();
                entry.get_write_handle()
            }
            .ok_or(FuseError::IsDir)?;
            let resource = self.resource.clone();
            let written = handle.write(offset, data, &resource).await?;
            Ok(ReplyWrite { written })
        }
    }
    fn access(
        &self,
        req: Request,
        inode: u64,
        mask: u32,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        async { Ok(()) }
    }
    fn fsync(
        &self,
        req: Request,
        inode: u64,
        fh: u64,
        datasync: bool,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        async { Ok(()) }
    }
    fn fsyncdir(
        &self,
        req: Request,
        inode: u64,
        fh: u64,
        datasync: bool,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        async { Ok(()) }
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
            let tree = self.tree.lock();
            let node = tree.get(inode as usize).ok_or(FuseError::InvaildIno)?;

            match node.get_value().kind() {
                FileType::Directory | FileType::NamedPipe | FileType::CharDevice => {
                    Err(FuseError::IsDir.into())
                }
                _ => Ok(()),
            }
        }
    }

    fn interrupt(&self, req: Request, unique: u64) -> impl Future<Output = FuseResult<()>> + Send {
        async { Ok(()) }
    }
    fn getattr(
        &self,
        req: Request,
        inode: u64,
        fh: Option<u64>,
        flags: u32,
    ) -> impl Future<Output = FuseResult<ReplyAttr>> + Send {
        async move {
            let tree = self.tree.lock();
            let entry = tree.get(inode as usize).ok_or(FuseError::InvaildIno)?;
            // FIXME: unsure about the inode
            Ok(reply_attr(entry.get_value(), inode))
        }
    }
}
