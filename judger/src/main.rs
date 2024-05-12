mod config;
mod error;
mod filesystem;
mod language;
mod sandbox;

pub use config::CONFIG;
type Result<T> = std::result::Result<T, error::Error>;

fn main() {}
