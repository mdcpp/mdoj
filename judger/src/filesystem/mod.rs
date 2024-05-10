//! Filesystem module that is mountable(actuall mount and
//! is accessible for user in this operation system)
mod adapter;
mod entry;
mod error;
mod resource;
mod table;

pub use entry::prelude::*;
