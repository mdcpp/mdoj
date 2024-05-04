// mod adapter;
mod entry;
mod macro_;
// mod overlay;
mod table;
mod tree;

#[derive(thiserror::Error, Debug)]
pub enum FuseError {
    #[error("io error")]
    IoError(#[from] std::io::Error),
    #[error("not a readable file")]
    NotReadable,
}
