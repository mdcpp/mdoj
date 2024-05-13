mod config;
mod error;
mod filesystem;
mod language;
mod sandbox;
mod server;

pub use config::CONFIG;
type Result<T> = std::result::Result<T, error::Error>;

fn main() {}
