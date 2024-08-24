use std::{ffi::OsStr, num::NonZeroU32, path::Path, sync::Arc};

use bytes::Bytes;
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

/// A synchronized stream from vector
type VecStream<I> = tokio_stream::Iter<std::vec::IntoIter<I>>;

// filesystem
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
            Entry::from_vec(content),
        );
    }
}

impl<F> raw::Filesystem for Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
{
    async fn init(&self, _: Request) -> FuseResult<ReplyInit> {
        Ok(ReplyInit {
            max_write: NonZeroU32::new(BLOCKSIZE as u32).unwrap(),
        })
    }
    async fn destroy(&self, _: Request) {}

    async fn lookup(&self, req: Request, parent: u64, name: &OsStr) -> FuseResult<ReplyEntry> {
        let tree = self.tree.lock();
        let parent_node = tree.get(parent as usize).ok_or(FuseError::InvalidIno)?;
        let node = parent_node
            .get_by_component(name)
            .ok_or(FuseError::InvalidPath)?;
        // FIXME: unsure about the inode
        Ok(reply_entry(&req, node.get_value(), node.get_id() as u64))
    }

    async fn forget(&self, _: Request, _: u64, _: u64) {}

    async fn getattr(
        &self,
        req: Request,
        inode: u64,
        _: Option<u64>,
        _: u32,
    ) -> FuseResult<ReplyAttr> {
        let tree = self.tree.lock();
        let node = tree.get(inode as usize).ok_or(FuseError::InvalidIno)?;
        // FIXME: unsure about the inode
        Ok(reply_attr(&req, node.get_value(), inode))
    }
    async fn setattr(
        &self,
        req: Request,
        inode: Inode,
        _: Option<u64>,
        _: SetAttr,
    ) -> FuseResult<ReplyAttr> {
        let tree = self.tree.lock();
        let node = tree.get(inode as usize).ok_or(FuseError::InvalidIno)?;
        Ok(reply_attr(&req, node.get_value(), inode))
    }
    async fn readlink(&self, _: Request, inode: Inode) -> FuseResult<ReplyData> {
        let tree = self.tree.lock();
        let node = tree.get(inode as usize).ok_or(FuseError::InvalidIno)?;
        let link = node
            .get_value()
            .get_symlink()
            .ok_or(FuseError::InvalidArg)?;
        Ok(ReplyData {
            data: Bytes::copy_from_slice(link.as_encoded_bytes()),
        })
    }
    async fn mkdir(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
        _: u32,
        _: u32,
    ) -> FuseResult<ReplyEntry> {
        let mut tree = self.tree.lock();
        let mut parent_node = tree.get_mut(parent as usize).ok_or(FuseError::InvalidIno)?;
        if parent_node.get_value().kind() != FileType::Directory {
            return Err(FuseError::NotDir.into());
        }
        let mut node = parent_node
            .insert(name.to_os_string(), Entry::Directory)
            .ok_or(FuseError::AlreadyExist)?;
        let ino = node.get_id() as u64;
        Ok(reply_entry(&req, node.get_value(), ino))
    }
    async fn unlink(&self, _: Request, parent: Inode, name: &OsStr) -> FuseResult<()> {
        let mut tree = self.tree.lock();
        let mut parent_node = tree.get_mut(parent as usize).ok_or(FuseError::InvalidIno)?;
        if parent_node.get_value().kind() != FileType::Directory {
            return Err(FuseError::NotDir.into());
        }
        parent_node.remove_by_component(name);
        Ok(())
    }
    async fn open(&self, _: Request, inode: u64, flags: u32) -> FuseResult<ReplyOpen> {
        // ignore write permission, because some application may open files
        // with write permission but never write
        let tree = self.tree.lock();
        let node = tree.get(inode as usize).ok_or(FuseError::InvalidIno)?;
        if node.get_value().kind() == FileType::Directory {
            return Err(FuseError::IsDir.into());
        }
        let mut entry = node.get_value().clone();
        entry.set_append().await;
        let fh = self.handle_table.add(AsyncMutex::new(entry));
        Ok(ReplyOpen { fh, flags })
    }
    async fn read(
        &self,
        _: Request,
        _: u64,
        fh: u64,
        offset: u64,
        size: u32,
    ) -> FuseResult<ReplyData> {
        let session = self.handle_table.get(fh).ok_or(FuseError::HandleNotFound)?;
        let mut lock = session.lock().await;

        if lock.kind() != FileType::RegularFile {
            return Err(FuseError::IsDir.into());
        }

        Ok(lock
            .read(offset, size)
            .await
            .map(|data| ReplyData { data })?)
    }
    async fn write(
        &self,
        _: Request,
        _: u64,
        fh: u64,
        offset: u64,
        data: &[u8],
        _: u32,
        _: u32,
    ) -> FuseResult<ReplyWrite> {
        let session = self
            .handle_table
            .get(fh)
            .ok_or(FuseError::HandleNotFound)
            .unwrap();
        let mut lock = session.lock().await;

        if lock.kind() != FileType::RegularFile {
            return Err(FuseError::IsDir.into());
        }

        Ok(lock
            .write(offset, data, &self.resource)
            .await
            .map(|written| ReplyWrite { written })
            .ok_or(FuseError::Unimplemented)?)
    }
    async fn statfs(&self, _: Request, _: u64) -> FuseResult<ReplyStatFs> {
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
    async fn release(
        &self,
        _: Request,
        _: u64,
        fh: u64,
        _: u32,
        _: u64,
        _: bool,
    ) -> FuseResult<()> {
        self.handle_table.remove(fh);
        Ok(())
    }
    async fn fsync(&self, _: Request, _: u64, _: u64, _: bool) -> FuseResult<()> {
        Ok(())
    }
    async fn flush(&self, _: Request, _: Inode, fh: u64, _: u64) -> FuseResult<()> {
        let node = self.handle_table.get(fh).ok_or(FuseError::HandleNotFound)?;
        // The result is intentionally ignored, from the behavior of `ld`,
        // we know that ld actually flush readonly file.
        Entry::flush(node).await;
        Ok(())
    }
    async fn opendir(&self, _: Request, inode: u64, flags: u32) -> FuseResult<ReplyOpen> {
        let tree = self.tree.lock();
        let node = tree.get(inode as usize).ok_or(FuseError::InvalidIno)?;
        if node.get_value().kind() != FileType::Directory {
            return Err(FuseError::NotDir.into());
        }
        let fh = self
            .handle_table
            .add(AsyncMutex::new(node.get_value().clone()));
        Ok(ReplyOpen { fh, flags })
    }
    type DirEntryStream<'a>=VecStream<FuseResult<DirectoryEntry>> where Self: 'a;
    async fn readdir(
        &self,
        _: Request,
        parent: u64,
        _: u64,
        offset: i64,
    ) -> FuseResult<ReplyDirectory<Self::DirEntryStream<'_>>> {
        let tree = self.tree.lock();
        let node = tree.get(parent as usize).ok_or(FuseError::InvalidIno)?;

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
    async fn fsyncdir(&self, _: Request, _: u64, _: u64, _: bool) -> FuseResult<()> {
        Ok(())
    }
    async fn access(&self, _: Request, _: u64, _: u32) -> FuseResult<()> {
        // FIXME: only allow current user to access
        Ok(())
    }
    async fn create(
        &self,
        req: Request,
        parent: u64,
        name: &OsStr,
        _: u32,
        flags: u32,
    ) -> FuseResult<ReplyCreated> {
        let mut tree = self.tree.lock();
        let mut parent_node = tree.get_mut(parent as usize).ok_or(FuseError::InvalidIno)?;
        if parent_node.get_value().kind() != FileType::Directory {
            return Err(FuseError::NotDir.into());
        }
        let mut node = parent_node
            .insert(name.to_os_string(), Entry::new_file())
            .ok_or(FuseError::AlreadyExist)?;

        let entry = node.get_value().clone();
        let fh = self.handle_table.add(AsyncMutex::new(entry));
        let inode = node.get_id() as u64;

        Ok(reply_created(&req, node.get_value(), fh, flags, inode))
    }
    async fn interrupt(&self, _: Request, _: u64) -> FuseResult<()> {
        Ok(())
    }
    async fn fallocate(
        &self,
        _: Request,
        inode: u64,
        _: u64,
        _: u64,
        _: u64,
        _: u32,
    ) -> FuseResult<()> {
        let tree = self.tree.lock();
        let node = tree.get(inode as usize).ok_or(FuseError::InvalidIno)?;

        match node.get_value().kind() {
            FileType::Directory | FileType::NamedPipe | FileType::CharDevice => {
                Err(FuseError::IsDir.into())
            }
            _ => Ok(()),
        }
    }
    type DirEntryPlusStream<'a>=VecStream<FuseResult<DirectoryEntryPlus>> where Self: 'a;
    async fn readdirplus(
        &self,
        req: Request,
        parent: u64,
        _: u64,
        offset: u64,
        _: u64,
    ) -> FuseResult<ReplyDirectoryPlus<Self::DirEntryPlusStream<'_>>> {
        let tree = self.tree.lock();
        let node = tree.get(parent as usize).ok_or(FuseError::InvalidIno)?;

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

    #[allow(clippy::declare_interior_mutable_const)]
    const UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);

    async fn nested_tar() -> Filesystem<File> {
        let template = Template::new("test/nested.tar").await.unwrap();
        template.as_filesystem(1024 * 1024)
    }
    fn spawn_request() -> Request {
        Request {
            #[allow(clippy::declare_interior_mutable_const)]
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
