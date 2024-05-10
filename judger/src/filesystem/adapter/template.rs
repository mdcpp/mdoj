use std::path::Path;

use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek},
};

use crate::filesystem::entry::TarTree;

use super::fuse::Filesystem;

pub struct Template<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    tree: TarTree<F>,
}

impl<F> Template<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
{
    pub fn new_inner(tree: TarTree<F>) -> Self {
        Self { tree }
    }
    pub fn as_filesystem(&self, permit: u64) -> Filesystem<F> {
        Filesystem::new(self.tree.clone(), permit)
    }
    pub async fn read_by_path(&self, path: impl AsRef<Path>) -> Option<Vec<u8>> {
        self.tree.read_by_path(path).await
    }
}

impl Template<File> {
    pub async fn new(path: impl AsRef<Path> + Clone) -> std::io::Result<Self> {
        let tree = TarTree::new(path).await?;
        Ok(Self::new_inner(tree))
    }
}
