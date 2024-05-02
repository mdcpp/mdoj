//! collection of entry
//!
//! In tar file, structure is like this:
//! | type | content | ...
//!
//! And we map each type of content to BTreeMap<PathBuf, Entry>

use std::ffi::OsString;

use tokio::io::{AsyncRead, AsyncSeek};

use super::block::TarBlock;

/// Entry from tar file, should be readonly
#[derive(Debug, Default)]
pub enum Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    SymLink(OsString),
    HardLink(u64),
    #[default]
    Directory,
    File(TarBlock<F>),
}

impl<F> Clone for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::SymLink(arg0) => Self::SymLink(arg0.clone()),
            Self::HardLink(arg0) => Self::HardLink(arg0.clone()),
            Self::Directory => Self::Directory,
            Self::File(arg0) => Self::File(arg0.clone()),
        }
    }
}

impl<F> PartialEq for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SymLink(l0), Self::SymLink(r0)) => l0 == r0,
            (Self::File(l0), Self::File(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
