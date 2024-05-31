use std::{ffi::OsStr, num::NonZeroU32, path::Path, sync::Arc};

use bytes::Bytes;
use futures_core::Future;
use spin::Mutex;
use tokio::io::{AsyncRead, AsyncSeek};
use tokio::sync::Mutex as AsyncMutex;

use crate::filesystem::entry::{Entry, BLOCKSIZE};
use crate::filesystem::resource::Resource;
use crate::filesystem::table::{to_internal_path, AdjTable};

use super::{error::FuseError, handle::HandleTable, reply::*};
use fuse3::{
    raw::{reply::*, *},
    Result as FuseResult, *,
};

/// A asynchorized stream from vector
type VecStream<I> = tokio_stream::Iter<std::vec::IntoIter<I>>;

// filesystem is an adapter, it should not contain any business logic.
pub struct Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    handle_table: HandleTable<AsyncMutex<Entry<F>>>,
    tree: Mutex<AdjTable<Entry<F>>>,
    resource: Arc<Resource>,
}

impl<F> Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
{
    /// Create a new filesystem
    pub(super) fn new(tree: AdjTable<Entry<F>>, fs_size: u64) -> Self {
        Self {
            handle_table: HandleTable::new(),
            tree: Mutex::new(tree),
            resource: Arc::new(Resource::new(fs_size)),
        }
    }
    /// Mount the filesystem to a path,
    /// return a raw handle from `libfuse`
    pub async fn raw_mount_with_path(
        self,
        path: impl AsRef<Path> + Clone,
    ) -> std::io::Result<MountHandle> {
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };

        let mut mount_options = MountOptions::default();

        mount_options.uid(uid).gid(gid).force_readdir_plus(true);

        Session::new(mount_options)
            .mount_with_unprivileged(self, path.as_ref())
            .await
    }
    /// Insert a file by path before actual mounts.
    pub fn insert_by_path(&self, path: impl AsRef<Path>, content: Vec<u8>) {
        let mut tree = self.tree.lock();
        tree.insert_by_path(
            to_internal_path(path.as_ref()),
            || Entry::Directory,
            Entry::new_file_with_vec(content),
        );
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
            let parent_node = tree.get(parent as usize).ok_or(FuseError::InvaildIno)?;
            let node = parent_node
                .get_by_component(name)
                .ok_or(FuseError::InvalidPath)?;
            // FIXME: unsure about the inode
            Ok(reply_entry(&req, node.get_value(), node.get_id() as u64))
        }
    }
    fn forget(&self, _: Request, inode: u64, _: u64) -> impl Future<Output = ()> + Send {
        async {}
    }
    fn release(
        &self,
        req: Request,
        inode: u64,
        fh: u64,
        flags: u32,
        lock_owner: u64,
        flush: bool,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        async move {
            self.handle_table.remove(fh);
            Ok(())
        }
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
            let fh = self
                .handle_table
                .add(AsyncMutex::new(node.get_value().clone()));
            Ok(ReplyOpen { fh, flags })
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
            let node = tree.get(inode as usize).ok_or(FuseError::InvaildIno)?;
            if node.get_value().kind() == FileType::Directory {
                return Err(FuseError::IsDir.into());
            }
            let fh = self
                .handle_table
                .add(AsyncMutex::new(node.get_value().clone()));
            Ok(ReplyOpen { fh, flags })
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

            let parent_node = node.parent().unwrap_or_else(|| tree.get_first());

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
                    .filter_map(|inode| {
                        let node = tree.get(inode).unwrap();
                        Some(dir_entry(
                            node.get_name()?.to_os_string(),
                            node.get_value(),
                            inode as u64,
                        ))
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

            let parent_node = node.parent().unwrap_or_else(|| tree.get_first());

            let entries = vec![
                Ok(dir_entry_plus(
                    &req,
                    OsStr::new(".").to_os_string(),
                    node.get_value(),
                    node.get_id() as u64,
                    1,
                )),
                Ok(dir_entry_plus(
                    &req,
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
                    .filter_map(|(offset, inode)| {
                        let node = tree.get(inode).unwrap();
                        Some(dir_entry_plus(
                            &req,
                            node.get_name()?.to_os_string(),
                            node.get_value(),
                            inode as u64,
                            (offset + 3) as i64,
                        ))
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
            let session = self.handle_table.get(fh).ok_or(FuseError::HandleNotFound)?;
            let mut lock = session.lock().await;
            Ok(lock
                .read(offset, size)
                .await
                .ok_or(Into::<Errno>::into(FuseError::IsDir))?
                .map(|data| ReplyData { data })?)
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
            let session = self
                .handle_table
                .get(fh)
                .ok_or(FuseError::HandleNotFound)
                .unwrap();

            Ok(Entry::write(session, offset, data, &self.resource)
                .await
                .ok_or_else(|| Into::<Errno>::into(FuseError::IsDir))?
                .map(|written| ReplyWrite { written })?)
        }
    }
    fn flush(
        &self,
        req: Request,
        inode: Inode,
        fh: u64,
        lock_owner: u64,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        async move {
            let node = self.handle_table.get(fh).ok_or(FuseError::HandleNotFound)?;
            Entry::flush(node).await.ok_or(FuseError::Unimplemented);
            Ok(())
        }
    }
    fn access(
        &self,
        req: Request,
        inode: u64,
        mask: u32,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        // FIXME: only allow current user to access
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
            let node = tree.get(inode as usize).ok_or(FuseError::InvaildIno)?;
            // FIXME: unsure about the inode
            Ok(reply_attr(&req, node.get_value(), inode))
        }
    }
    fn setattr(
        &self,
        req: Request,
        inode: Inode,
        fh: Option<u64>,
        set_attr: SetAttr,
    ) -> impl Future<Output = FuseResult<ReplyAttr>> + Send {
        async move {
            let tree = self.tree.lock();
            let node = tree.get(inode as usize).ok_or(FuseError::InvaildIno)?;
            Ok(reply_attr(&req, node.get_value(), inode))
        }
    }
    // open and create fd
    fn create(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        flags: u32,
    ) -> impl Future<Output = FuseResult<ReplyCreated>> + Send {
        async move {
            let mut tree = self.tree.lock();
            let mut parent_node = tree.get_mut(parent as usize).ok_or(FuseError::InvaildIno)?;
            if parent_node.get_value().kind() != FileType::Directory {
                return Err(FuseError::NotDir.into());
            }
            let mut node = parent_node
                .insert(name.to_os_string(), Entry::new_file())
                .ok_or(FuseError::AlreadyExist)?;

            let mut entry=node.get_value().clone();
            if flags&u32::from_ne_bytes(libc::O_APPEND.to_ne_bytes()) != 0 {
                entry.set_append().await;
            }

            let fh = self
                .handle_table
                .add(AsyncMutex::new(entry));

            let inode = node.get_id() as u64;
            let entry = node.get_value();
            Ok(reply_created(&req, entry, fh, flags, inode))
        }
    }
    fn mkdir(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> impl Future<Output = FuseResult<ReplyEntry>> + Send {
        async move {
            let mut tree = self.tree.lock();
            let mut parent_node = tree.get_mut(parent as usize).ok_or(FuseError::InvaildIno)?;
            if parent_node.get_value().kind() != FileType::Directory {
                return Err(FuseError::NotDir.into());
            }
            let mut node = parent_node
                .insert(name.to_os_string(), Entry::Directory)
                .ok_or(FuseError::AlreadyExist)?;
            let ino = node.get_id() as u64;
            Ok(reply_entry(&req, node.get_value(), ino))
        }
    }
    fn readlink(
        &self,
        req: Request,
        inode: Inode,
    ) -> impl Future<Output = FuseResult<ReplyData>> + Send {
        async move {
            let tree = self.tree.lock();
            let node = tree.get(inode as usize).ok_or(FuseError::InvaildIno)?;
            let link = node
                .get_value()
                .get_symlink()
                .ok_or(FuseError::InvialdArg)?;
            Ok(ReplyData {
                data: Bytes::copy_from_slice(link.as_encoded_bytes()),
            })
        }
    }
    fn unlink(
        &self,
        req: Request,
        parent: Inode,
        name: &OsStr,
    ) -> impl Future<Output = FuseResult<()>> + Send {
        async move {
            let mut tree = self.tree.lock();
            let mut parent_node = tree.get_mut(parent as usize).ok_or(FuseError::InvaildIno)?;
            if parent_node.get_value().kind() != FileType::Directory {
                return Err(FuseError::NotDir.into());
            }
            parent_node.remove_by_component(name);
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        ffi::OsStr,
        sync::atomic::{AtomicU64, Ordering},
    };

    use fuse3::{
        raw::{Filesystem as _, Request},
        Errno,
    };
    use tokio::fs::File;

    use crate::filesystem::adapter::Template;

    use super::Filesystem;

    const UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);

    async fn nested_tar() -> Filesystem<File> {
        let template = Template::new("test/nested.tar").await.unwrap();
        template.as_filesystem(1024 * 1024)
    }
    fn spawn_request() -> Request {
        Request {
            unique: UNIQUE_COUNTER.fetch_add(1, Ordering::AcqRel),
            uid: 1000,
            gid: 1000,
            pid: 2,
        }
    }

    #[tokio::test]
    async fn lookup() {
        let fs = nested_tar().await;
        assert_eq!(
            fs.lookup(spawn_request(), 1, OsStr::new("nest"))
                .await
                .unwrap()
                .attr
                .ino,
            2
        );
        assert_eq!(
            fs.lookup(spawn_request(), 1, OsStr::new("o.txt"))
                .await
                .unwrap()
                .attr
                .ino,
            5
        );
        assert_eq!(
            fs.lookup(spawn_request(), 2, OsStr::new("a.txt"))
                .await
                .unwrap()
                .attr
                .ino,
            3
        );
        assert_eq!(
            fs.lookup(spawn_request(), 2, OsStr::new("o.txt"))
                .await
                .unwrap_err(),
            Errno::new_not_exist()
        );
        assert_eq!(
            fs.lookup(spawn_request(), 100, OsStr::new("o.txt"))
                .await
                .unwrap_err(),
            libc::ENOENT.into()
        )
    }
}
