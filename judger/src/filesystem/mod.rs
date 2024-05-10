//! Filesystem module that is mountable(actuall mount and
//! is accessible for user in this operation system)
mod adapter;
mod table;
mod entry;
mod error;
mod resource;

pub use entry::prelude::*;
