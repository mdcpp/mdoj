use std::{ffi::OsString, io::Read, os::unix::ffi::OsStringExt, path::Path, sync::Arc};

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

pub struct EntryTree<F>(pub AdjTable<Entry<F>>)
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static;

impl<F> Clone for EntryTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<F> Default for EntryTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn default() -> Self {
        let mut tree = AdjTable::new();
        tree.insert_root(Entry::Directory);
        Self(tree)
    }
}

impl<F> EntryTree<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
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
            x => {
                log::warn!("unsupported entry type: {:?}", x);
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

impl EntryTree<File> {
    pub async fn new(path: impl AsRef<Path> + Clone) -> Result<Self> {
        let file = File::open(path.clone()).await?;
        let std_file = File::open(path).await?.into_std().await;
        Self::inner_new(file, std_file).await
    }
}

#[cfg(test)]
impl<T> EntryTree<BufReader<Cursor<T>>>
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
                .0
                .get_by_path(to_internal_path(Path::new($path)))
                .unwrap();
            let entry = node;
            assert_eq!(entry.get_value().kind(), FileType::$kind);
        }};
    }

    #[tokio::test]
    async fn nested_map() {
        let content = include_bytes!("../../../test/nested.tar");

        let tree = EntryTree::test_new(content).await.unwrap();

        assert_kind!(tree, "nest", Directory);
        assert_kind!(tree, "nest/a.txt", RegularFile);
        assert_kind!(tree, "nest/b.txt", RegularFile);
        assert_kind!(tree, "o.txt", RegularFile);
    }
}
