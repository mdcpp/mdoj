use std::path::Path;

use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek},
};

use crate::filesystem::{
    entry::{Entry, TarTree},
    table::AdjTable,
};

use super::fuse::Filesystem;

pub struct Template<F>(AdjTable<Entry<F>>)
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static;

impl<F> Template<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
{
    /// use template to create a filesystem
    pub fn as_filesystem(&self, permit: u64) -> Filesystem<F> {
        Filesystem::new(self.0.clone(), permit)
    }
    /// read a file by path
    pub async fn read_by_path(&self, path: impl AsRef<Path>) -> Option<Vec<u8>> {
        self.read_by_path(path).await
    }
}

impl Template<File> {
    /// Create a new template from a tar file
    pub async fn new(path: impl AsRef<Path> + Clone) -> std::io::Result<Self> {
        let tree = TarTree::new(path).await?;
        Ok(Self(tree.0))
    }
}
