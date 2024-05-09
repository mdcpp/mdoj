//! Filesystem module that is mountable(actuall mount and
//! is accessible for user in this operation system)
mod adapter;
mod adj;
mod entry;
mod error;
mod macro_;
mod resource;

pub use entry::prelude::*;
