use std::path::Path;

use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek},
};

use crate::{
    filesystem::{table::DeepClone, TarTree},
    semaphore::Permit,
};

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
    pub async fn as_filesystem(&self, permit: Permit) -> Filesystem<F> {
        Filesystem::new(self.tree.clone(), permit)
    }
}

impl Template<File> {
    pub async fn new(path: impl AsRef<Path> + Clone) -> std::io::Result<Self> {
        let tree = TarTree::new(path).await?;
        Ok(Self::new_inner(tree))
    }
}
