//! Filesystem module that is mountable(actuall mount and
//! is accessible for user in this operation system)
mod adapter;
mod entry;
mod error;
mod mkdtemp;
mod resource;
mod table;

pub use adapter::{Filesystem, Template};
pub use fuse3::raw::MountHandle;
