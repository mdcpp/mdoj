mod block;
mod entry;
mod reply;

pub use block::MEMBLOCK_BLOCKSIZE as BLOCKSIZE;
pub use entry::*;
pub use reply::Parsable;
use tokio::io::{AsyncRead, AsyncSeek};

use super::{
    table::{HandleTable, INodeTable},
    tar::TarLayer,
};

pub struct Overlay<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    tar: TarLayer<F>,
    pub inode: INodeTable<Entry<F>>,
    pub handle: HandleTable<Entry<F>>,
}

impl<F> Overlay<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    pub async fn new(tar: TarLayer<F>) -> Self {
        let inode = INodeTable::new();
        inode.add_entry_ro(tar.get_root().await);
        Self {
            tar,
            inode,
            handle: HandleTable::new(),
        }
    }
    pub fn get_root(&self) -> Entry<F> {
        self.inode.get(1).unwrap()
    }
    pub fn lookup(&self, inode: u64) -> Option<Entry<F>> {
        self.inode.get(inode)
    }
}
