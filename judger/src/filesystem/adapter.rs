use std::num::NonZeroU32;

use crate::semaphore::Semaphore;
use fuse3::{
    raw::{reply::*, Request},
    Errno, Result as FuseResult,
};
use std::future::{ready as future_ready, Future};
use tokio::io::{AsyncRead, AsyncSeek};

use super::{
    overlay::{Overlay},
    table::HandleTable,
    tree::ArcNode,
};

type VecStream<I> = tokio_stream::Iter<std::iter::Cloned<std::slice::Iter<'static, I>>>;

type INODE = u64;
type HANDLE = u64;

// fn to_attr(inode: u64) -> FileAttr {
//     FileAttr {
//         ino: inode,
//         size: todo!(),
//         blocks: todo!(),
//         atime: todo!(),
//         mtime: todo!(),
//         ctime: todo!(),
//         kind: todo!(),
//         perm: todo!(),
//         nlink: todo!(),
//         uid: todo!(),
//         gid: todo!(),
//         rdev: todo!(),
//         blksize: todo!(),
//     }
// }

pub struct Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    handle_table: HandleTable<ArcNode<Entry<F>>>,
    overlay: Overlay<F>,
    semaphore: Semaphore,
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
        name: &std::ffi::OsStr,
    ) -> impl Future<Output = FuseResult<ReplyEntry>> + Send {
        async move {
            match self
                .overlay
                .inode
                .get_child_by_componment(parent, name)
                .await
            {
                Some(x) => todo!(),
                None => Err(Errno::new_not_exist()),
            }
        }
    }
    fn forget(
        &self,
        _: Request,
        inode: u64,
        _: u64,
    ) -> impl core::future::Future<Output = ()> + Send {
        self.overlay.inode.remove(inode);
        future_ready(())
    }
    fn statfs(
        &self,
        _: Request,
        inode: u64,
    ) -> impl Future<Output = FuseResult<ReplyStatFs>> + Send {
        // FIXME: report files in directory
        async {
            Ok(ReplyStatFs {
                blocks: 0,
                bfree: 4096 * 4096,
                bavail: 4096 * 2048,
                files: 0,
                ffree: self.overlay.inode.get_free_inode(),
                bsize: BLOCKSIZE as u32,
                namelen: 256,
                frsize: BLOCKSIZE as u32,
            })
        }
    }
}
