//! A module that provides a way to setup environment for a process and run.
//!
//! Using this module should be SAFE(can't launching a process without
//! explicit resource limitation)
//!
//! ```norun
//! use process::*;
//!
//! // implement process context yourself
//! let ctx=Context::new();
//!
//! let process=Process::new(ctx).unwrap();
//! let corpse=process.wait(b"data for stdin").await.unwrap();
//! ```

mod corpse;
mod nsjail;
mod process;

use super::*;
