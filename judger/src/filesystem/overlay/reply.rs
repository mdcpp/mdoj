use std::time::Duration;

use fuse3::{
    raw::{reply::*, Request},
    Timestamp,
};
use tokio::io::{AsyncRead, AsyncSeek};

use super::Entry;

pub trait Parsable<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn parse(request: Request, entry: Entry<F>) -> Self;
}

impl<F> Parsable<F> for ReplyEntry
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn parse(request: Request, entry: Entry<F>) -> Self {
        Self {
            ttl: Duration::from_secs(30),
            attr: FileAttr {
                ino: entry.get_inode(),
                size: 0,
                blocks: 0,
                atime: Timestamp::new(0, 0),
                mtime: Timestamp::new(0, 0),
                ctime: Timestamp::new(0, 0),
                kind: todo!(),
                perm: todo!(),
                nlink: todo!(),
                uid: todo!(),
                gid: todo!(),
                rdev: todo!(),
                blksize: todo!(),
            },
            generation: 1,
        }
    }
}
