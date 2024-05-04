use super::prelude::ReadEntry;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::Read,
    os::unix::ffi::OsStringExt,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use spin::rwlock::RwLock;
#[cfg(test)]
use std::io::Cursor;
use tar::{Archive, EntryType};
// use tar::*;
#[cfg(test)]
use tokio::io::BufReader;
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek},
    sync::Mutex,
    task::spawn_blocking,
};

use crate::{
    filesystem::{
        table::{InodeHandle, InodeTable},
        tree::{arc_lock, ArcNode, InsertResult, Node, Tree},
    },
    Error,
};

use super::{ro::TarBlock, Entry, InoEntry};

impl<F> InodeTable<ArcNode<InoEntry<F>>>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn new_read_entry(&self, entry: ReadEntry<F>) -> ArcNode<InoEntry<F>> {
        let handle = self.allocate();
        let node = Node::new(InoEntry {
            entry: Entry::Read(entry),
            inode: handle.get_inode(),
        });
        handle.consume(node.clone());
        node
    }
}

pub struct TarTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    pub tree: Tree<InoEntry<F>>,
    pub inode: InodeTable<ArcNode<InoEntry<F>>>,
}

impl<F> Default for TarTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn default() -> Self {
        let inode = InodeTable::default();
        let handle = inode.allocate_root();
        let root = Node::new(InoEntry {
            entry: Entry::Read(ReadEntry::new_dir()),
            inode: handle.get_inode(),
        });
        let tree = Tree::new(root.clone());
        handle.consume(root);
        Self { tree, inode }
    }
}

impl<F> TarTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    async fn parse_entry<R: Read>(
        &mut self,
        entry: tar::Entry<'_, R>,
        file: &Arc<Mutex<F>>,
    ) -> Result<(), Error> {
        let path = entry.path()?;
        let read_entry = match entry.header().entry_type() {
            EntryType::Regular | EntryType::Continuous => {
                let start = entry.raw_file_position();
                let size = entry.size();
                ReadEntry::new_file(TarBlock::new(file.clone(), start, size))
            }
            EntryType::Symlink => ReadEntry::new_symlink(OsString::from_vec(
                entry.link_name_bytes().unwrap().into_owned(),
            )),
            EntryType::Directory => ReadEntry::new_dir(),
            _ => {
                log::warn!("unsupported entry type: {:?}", entry.header().entry_type());
                return Ok(());
            }
        };

        let node = self.inode.new_read_entry(read_entry);
        match self
            .tree
            .insert_path_recursive(path, node, || {
                self.inode.new_read_entry(ReadEntry::new_dir())
            })
            .await
        {
            InsertResult::AlreadyExists(_) => Err(Error::InvalidTarball("duplicated entry")),
            InsertResult::ParentNotFound => unreachable!(),
            _ => Ok(()),
        }
    }
    // FIXME: this block
    pub async fn inner_new(file: F, std_file: impl Read + Send + 'static) -> Result<Self, Error> {
        let mut archive = Archive::new(std_file);
        let file = Arc::new(Mutex::new(file));

        let mut self_ = Self::default();
        let entries = archive.entries()?;
        for entry in entries {
            self_.parse_entry(entry?, &file).await?;
        }
        Ok(self_)
    }
}

impl TarTree<File> {
    pub async fn new(path: impl AsRef<Path> + Clone) -> Result<Self, Error> {
        let file = File::open(path.clone()).await?;
        let std_file = File::open(path).await?.into_std().await;
        Self::inner_new(file, std_file).await
    }
}

#[cfg(test)]
impl<T> TarTree<BufReader<Cursor<T>>>
where
    T: AsRef<[u8]> + Send + Unpin + Clone + 'static,
{
    pub async fn test_new(content: T) -> Result<Self, Error> {
        let file = BufReader::new(Cursor::new(content.clone()));
        let std_file = BufReader::new(Cursor::new(content)).into_inner();
        Self::inner_new(file, std_file).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use fuse3::FileType;
    use std::{ffi::OsStr, ops::Deref};
    use tokio::io::AsyncReadExt;

    macro_rules! assert_content {
        ($entry:expr, $content:expr) => {
            if let Entry::File(ref mut file) = $entry {
                let mut buf = vec![0_u8; $content.len()];
                file.read_exact(&mut buf).await.unwrap();
                assert_eq!(&buf, $content);
            } else {
                panic!("entry is not a file")
            }
        };
    }
    macro_rules! assert_kind {
        ($tree:expr,$path:expr, $kind:ident) => {{
            let node = $tree.tree.get_by_path($path).await.unwrap();
            let locked = node.read().await;
            let entry = locked.deref().deref();
            assert_eq!(entry.kind().await, FileType::$kind, "entry: {:?}", entry);
        }};
    }

    // #[tokio::test]
    // async fn single_file_map() {
    //     let content = include_bytes!("../../../test/single_file.tar");

    //     let tree = TarTree::test_new(content).await.unwrap();

    //     let file=tree.tree.get_root().read().await.get_by_component(OsStr::new("single_file.txt")).unwrap();

    //     let single_file = map.0.get_mut(OsStr::new("single_file.txt")).unwrap();

    //     assert_content!(single_file, b"hello world");
    // }
    #[tokio::test]
    async fn nested_map() {
        let content = include_bytes!("../../../test/nested.tar");

        let tree = TarTree::test_new(content).await.unwrap();

        assert_kind!(tree, "nest", Directory);
        assert_kind!(tree, "nest/a.txt", RegularFile);
        assert_kind!(tree, "nest/b.txt", RegularFile);
        assert_kind!(tree, "o.txt", RegularFile);
        // assert_content!(map.tree.get_mut(OsStr::new("nest/a.txt")).unwrap(), b"a");
        // assert_content!(map.tree.get_mut(OsStr::new("nest/b.txt")).unwrap(), b"b");
        // assert_content!(map.tree.get_mut(OsStr::new("o.txt")).unwrap(), b"o");
    }
}
