mod adapter;
mod entry;
mod macro_;
mod reply;
mod table;
mod tree;

pub use entry::prelude::*;

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
}

impl From<FuseError> for fuse3::Errno {
    fn from(value: FuseError) -> Self {
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
