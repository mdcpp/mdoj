//! Filesystem module that is mountable(actuall mount and
//! is accessible for user in this operation system)
//!
//!

mod adapter;
mod entry;
mod macro_;
mod reply;
mod table;
mod tree;

pub use entry::prelude::*;
use tokio::sync::broadcast::error;

#[derive(thiserror::Error, Debug)]
pub enum FuseError {
    #[error("not a readable file")]
    IsDir,
    #[error("end of file")]
    Eof,
    #[error("not a dir")]
    NotDir,
    #[error("out of resource")]
    OutOfPermit,
    #[error("number too large")]
    OutOfRange,
    #[error("unimplemented")]
    Unimplemented,
    #[error("missed inode")]
    InodeNotFound,
    #[error("missed handle")]
    HandleNotFound,
    #[error("underlaying file error")]
    Underlaying,
}

impl From<FuseError> for fuse3::Errno {
    fn from(value: FuseError) -> Self {
        log::warn!("FUSE driver broken: {}", value);
        match value {
            FuseError::IsDir => libc::EISDIR,
            FuseError::NotDir => libc::ENOTDIR,
            FuseError::Eof => libc::EOF,
            FuseError::OutOfPermit => libc::ENOMEM,
            _ => {
                log::warn!("FUSE driver broken: {}", value);
                libc::ENOMEM
            }
        }
        .into()
    }
}
