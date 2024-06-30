//! collection of function that fill the value of
//! reply packets back to fuse connection
use std::{ffi::OsString, time::Duration};

use fuse3::{
    raw::{reply::*, Request},
    Timestamp,
};
use tokio::io::{AsyncRead, AsyncSeek};

use crate::filesystem::{entry::Entry, entry::BLOCKSIZE};

const TTL: Duration = Duration::from_secs(0);

pub fn dir_entry_plus<F>(
    req: &Request,
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
        attr: file_attr(req, entry, inode),
        entry_ttl: TTL,
        attr_ttl: TTL,
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

pub fn reply_attr<F>(req: &Request, entry: &Entry<F>, inode: u64) -> ReplyAttr
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    ReplyAttr {
        ttl: TTL,
        attr: file_attr(req, entry, inode),
    }
}

pub fn reply_entry<F>(req: &Request, entry: &Entry<F>, inode: u64) -> ReplyEntry
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    ReplyEntry {
        ttl: TTL,
        attr: file_attr(req, &entry, inode),
        generation: 0,
    }
}

pub fn file_attr<F>(req: &Request, entry: &Entry<F>, inode: u64) -> FileAttr
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
        perm: (libc::S_IRWXO | libc::S_IRWXG | libc::S_IRWXU) as u16,
        nlink: 1,
        uid: req.uid,
        gid: req.gid,
        rdev: 179 << 16 + 02,
        blksize: BLOCKSIZE as u32,
    }
}

pub fn reply_created<F>(
    req: &Request,
    entry: &Entry<F>,
    fh: u64,
    flags: u32,
    inode: u64,
) -> ReplyCreated
where
    F: AsyncRead + AsyncSeek + Send + Unpin + 'static,
{
    ReplyCreated {
        ttl: TTL,
        attr: file_attr(req, entry, inode),
        generation: 0,
        fh,
        flags,
    }
}
