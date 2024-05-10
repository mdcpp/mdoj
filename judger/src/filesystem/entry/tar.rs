use std::{
    ffi::OsString,
    io::Read,
    ops::{Deref, DerefMut},
    os::unix::ffi::OsStringExt,
    path::Path,
    sync::Arc,
};

#[cfg(test)]
use std::io::Cursor;
use tar::{Archive, EntryType};
#[cfg(test)]
use tokio::io::BufReader;
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek, Result},
    sync::Mutex,
};

use crate::filesystem::table::{to_internal_path, AdjTable};

use super::{ro::TarBlock, Entry};

pub struct TarTree<F>(AdjTable<Entry<F>>)
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static;

impl<F> Clone for TarTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<F> DerefMut for TarTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<F> Deref for TarTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    type Target = AdjTable<Entry<F>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<F> Default for TarTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn default() -> Self {
        let mut tree = AdjTable::new();
        tree.insert_root(Entry::Directory);
        Self(tree)
    }
}

impl<F> TarTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    pub async fn read_by_path(&self, path: impl AsRef<Path>) -> Option<Vec<u8>> {
        let node = self.0.get_by_path(to_internal_path(path.as_ref()))?;
        Some(node.get_value().read_all().await.unwrap())
    }
    async fn parse_entry<R: Read>(
        &mut self,
        entry: tar::Entry<'_, R>,
        file: &Arc<Mutex<F>>,
    ) -> Result<()> {
        let path = entry.path()?;
        let entry = match entry.header().entry_type() {
            EntryType::Regular | EntryType::Continuous => {
                let start = entry.raw_file_position();
                let size = entry.size();
                Entry::TarFile(TarBlock::new(file.clone(), start, size as u32))
            }
            EntryType::Symlink => Entry::SymLink(OsString::from_vec(
                entry.link_name_bytes().unwrap().into_owned(),
            )),
            EntryType::Directory => Entry::Directory,
            _ => {
                log::warn!("unsupported entry type: {:?}", entry.header().entry_type());
                return Ok(());
            }
        };

        self.0
            .insert_by_path(to_internal_path(&path), || Entry::Directory, entry);
        Ok(())
    }
    // FIXME: this block
    pub async fn inner_new(file: F, std_file: impl Read + Send + 'static) -> Result<Self> {
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
    pub async fn new(path: impl AsRef<Path> + Clone) -> Result<Self> {
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
    pub async fn test_new(content: T) -> Result<Self> {
        let file = BufReader::new(Cursor::new(content.clone()));
        let std_file = BufReader::new(Cursor::new(content)).into_inner();
        Self::inner_new(file, std_file).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use fuse3::FileType;

    macro_rules! assert_kind {
        ($tree:expr,$path:expr, $kind:ident) => {{
            let node = $tree
                .get_by_path(to_internal_path(Path::new($path)))
                .unwrap();
            let entry = node;
            assert_eq!(entry.get_value().kind(), FileType::$kind);
        }};
    }

    #[tokio::test]
    async fn nested_map() {
        let content = include_bytes!("../../../test/nested.tar");

        let tree = TarTree::test_new(content).await.unwrap();

        assert_kind!(tree, "nest", Directory);
        assert_kind!(tree, "nest/a.txt", RegularFile);
        assert_kind!(tree, "nest/b.txt", RegularFile);
        assert_kind!(tree, "o.txt", RegularFile);
    }
}
