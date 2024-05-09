use std::{ffi::OsString, time::Duration};

use fuse3::{
    raw::{reply::*, Request},
    Timestamp,
};
use tokio::io::{AsyncRead, AsyncSeek};

use crate::filesystem::{Entry, BLOCKSIZE};

pub fn dir_entry_plus<F>(
    name: OsString,
    entry: &Entry<F>,
    inode: u64,
    offset: i64,
) -> DirectoryEntryPlus
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    DirectoryEntryPlus {
        inode,
        generation: 0,
        kind: entry.kind(),
        name,
        offset,
        attr: file_attr(entry, inode),
        entry_ttl: Duration::from_secs(30),
        attr_ttl: Duration::from_secs(30),
    }
}

pub fn dir_entry<F>(name: OsString, entry: &Entry<F>, inode: u64) -> DirectoryEntry
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    DirectoryEntry {
        inode,
        kind: entry.kind(),
        name,
        offset: 1,
    }
}

pub fn reply_attr<F>(entry: &Entry<F>, inode: u64) -> ReplyAttr
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    ReplyAttr {
        ttl: Duration::from_secs(30),
        attr: file_attr(&entry, inode),
    }
}

pub fn reply_entry<F>(request: Request, entry: &Entry<F>, inode: u64) -> ReplyEntry
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    ReplyEntry {
        ttl: Duration::from_secs(30),
        attr: file_attr(&entry, inode),
        generation: 0,
    }
}

pub fn file_attr<F>(entry: &Entry<F>, inode: u64) -> FileAttr
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    FileAttr {
        ino: inode,
        size: entry.get_size(),
        blocks: 0,
        atime: Timestamp::new(0, 0),
        mtime: Timestamp::new(0, 0),
        ctime: Timestamp::new(0, 0),
        kind: entry.kind(),
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
