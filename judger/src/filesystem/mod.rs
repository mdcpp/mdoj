//! Filesystem module that is mountable(actually mount and
//! is accessible for user in this operating system)
mod adapter;
mod entry;
mod handle;
mod mkdtemp;
mod resource;
mod table;

pub use adapter::Template;
pub use handle::MountHandle;
