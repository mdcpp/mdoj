use std::{sync::Arc, time::Duration};

use fuse3::{
    raw::{reply::*, Request},
    Timestamp,
};
use tokio::io::{AsyncRead, AsyncSeek};

use super::{
    entry::{prelude::BLOCKSIZE, InoEntry},
    tree::ArcNode,
};

pub trait ImmutParsable<F>
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    async fn parse(request: Request, entry: ArcNode<InoEntry<F>>) -> Self;
}

impl<F> ImmutParsable<F> for ReplyEntry
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    #[inline]
    async fn parse(request: Request, entry: ArcNode<InoEntry<F>>) -> Self {
        let nlink = Arc::strong_count(&entry) - 1;
        let entry = entry.read().await;
        Self {
            ttl: Duration::from_secs(30),
            attr: FileAttr {
                ino: entry.inode,
                size: 0,
                blocks: 0,
                atime: Timestamp::new(0, 0),
                mtime: Timestamp::new(0, 0),
                ctime: Timestamp::new(0, 0),
                kind: entry.kind().await,
                perm: (libc::S_IREAD
                    | libc::S_IWRITE
                    | libc::S_IEXEC
                    | libc::S_IRWXU
                    | libc::S_IRWXO
                    | libc::S_ISVTX) as u16,
                nlink: nlink as u32,
                uid: request.gid,
                gid: request.uid,
                rdev: 179 << 16 + 02,
                blksize: BLOCKSIZE as u32,
            },
            generation: 1,
        }
    }
}
