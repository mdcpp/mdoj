//! Procedural for initialization
//!
//! This module is heavily couple with crate::server and require refactor

pub mod config;
pub mod db;
pub mod error;
pub mod logger;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
