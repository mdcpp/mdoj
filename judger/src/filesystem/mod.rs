//! Filesystem module that is mountable(actuallly mount and
//! is accessible for user in this operation system)
mod adapter;
mod entry;
mod handle;
mod mkdtemp;
mod resource;
mod table;

pub use adapter::{Filesystem, Template};
pub use handle::MountHandle;
