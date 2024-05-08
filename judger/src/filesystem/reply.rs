use std::{ffi::OsString, sync::Arc, time::Duration};

use fuse3::{
    raw::{reply::*, Request},
    Timestamp,
};
use tokio::io::{AsyncRead, AsyncSeek};

use super::{
    entry::{prelude::BLOCKSIZE, InoEntry},
    tree::ArcNode,
};

pub async fn dir_entry_plus<F>(
    parent_attr: FileAttr,
    name: OsString,
    entry: ArcNode<InoEntry<F>>,
) -> DirectoryEntryPlus
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    let entry = entry.read().await;
    DirectoryEntryPlus {
        inode: entry.inode,
        generation: 1,
        kind: entry.kind().await,
        name,
        offset: 1,
        attr: parent_attr,
        entry_ttl: Duration::from_secs(30),
        attr_ttl: Duration::from_secs(30),
    }
}

pub async fn dir_entry<F>(name: OsString, entry: ArcNode<InoEntry<F>>) -> DirectoryEntry
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    let entry = entry.read().await;
    DirectoryEntry {
        inode: entry.inode,
        kind: entry.kind().await,
        name,
        offset: 1,
    }
}

#[inline]
pub async fn reply_entry<F>(request: Request, entry: ArcNode<InoEntry<F>>) -> ReplyEntry
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    let nlink = Arc::strong_count(&entry) - 1;
    let entry = entry.read().await;
    ReplyEntry {
        ttl: Duration::from_secs(30),
        attr: file_attr(&entry).await,
        generation: 1,
    }
}

pub async fn file_attr<F>(entry: &InoEntry<F>) -> FileAttr
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    FileAttr {
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
        nlink: 1,
        uid: 0,
        gid: 0,
        rdev: 179 << 16 + 02,
        blksize: BLOCKSIZE as u32,
    }
}
