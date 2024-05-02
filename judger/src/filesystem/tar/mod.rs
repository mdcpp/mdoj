mod block;
mod entry;
mod map;

pub use entry::Entry;
use tokio::io::{AsyncRead, AsyncSeek};

use super::tree::{ArcNode, Tree};

pub struct TarLayer<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    map: Tree<Entry<F>>,
}

impl<F> TarLayer<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    pub async fn get_root(&self) -> ArcNode<Entry<F>> {
        self.map.get_by_path("/").await.unwrap()
    }
}
