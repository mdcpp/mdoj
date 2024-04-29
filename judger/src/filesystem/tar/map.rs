use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{Error, Read},
    path::Path,
    sync::Arc,
};

use tar::*;
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek},
    sync::Mutex,
    task::spawn_blocking,
};
#[cfg(test)]
use std::io::Cursor;
#[cfg(test)]
use tokio::io::BufReader;

use super::{block::TarBlock, entry::Entry};

pub struct TarMap<F>(BTreeMap<OsString, Entry<F>>)
where
    F: AsyncRead + AsyncSeek + Unpin + 'static;

impl<F> TarMap<F> where F: AsyncRead + AsyncSeek + Unpin + 'static {}

fn parse_entry<F, R>(
    map: &mut BTreeMap<OsString, Entry<F>>,
    entry: tar::Entry<'_, R>,
    file: &Arc<Mutex<F>>,
) -> Result<(), Error>
where
    F: AsyncRead + AsyncSeek + Unpin,
    R: Read,
{
    let path = entry.path()?;
    if let Some(link_path) = entry.link_name()? {
        map.insert(
            path.as_os_str().to_owned(),
            Entry::Link(Arc::new(link_path.as_os_str().to_owned())),
        );
    } else {
        let start = entry.raw_file_position();
        let size = entry.size();
        map.insert(
            path.as_os_str().to_owned(),
            Entry::File(TarBlock::new(file.clone(), start, size)),
        );
    }
    let mut ancestors = path.ancestors();
    ancestors.next();
    for ancestor in ancestors {
        if !map.contains_key(ancestor.as_os_str()) {
            map.insert(ancestor.as_os_str().to_owned(), Entry::Directory);
        }
    }

    Ok(())
}

impl<F> TarMap<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    pub async fn inner_new(file: F, std_file: impl Read + Send + 'static) -> Result<Self, Error> {
        let file = Arc::new(Mutex::new(file));

        let map = spawn_blocking(move || -> Result<_, Error> {
            let mut map = BTreeMap::new();

            let mut archive = Archive::new(std_file);
            let entries = archive.entries()?;
            for entry in entries {
                parse_entry(&mut map, entry?, &file)?;
            }

            Ok(map)
        })
        .await??;

        Ok(Self(map))
    }
}

impl TarMap<File> {
    pub async fn new(path: impl AsRef<Path> + Clone) -> Result<Self, Error> {
        let file = File::open(path.clone()).await?;
        let std_file = File::open(path).await?.into_std().await;
        Self::inner_new(file, std_file).await
    }
}

#[cfg(test)]
impl<T> TarMap<BufReader<Cursor<T>>>
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
    use std::ffi::OsStr;
    use tokio::io::AsyncReadExt;
    use super::*;

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

    #[tokio::test]
    async fn single_file_map() {
        let content = include_bytes!("../../../test/single_file.tar");

        let mut map = TarMap::test_new(content).await.unwrap();

        let single_file = map.0.get_mut(OsStr::new("single_file.txt")).unwrap();

        assert_content!(single_file, b"hello world");
    }
    #[tokio::test]
    async fn nested_map() {
        let content = include_bytes!("../../../test/nested.tar");

        let mut map = TarMap::test_new(content).await.unwrap();

        assert_eq!(*map.0.get(OsStr::new("nest")).unwrap(), Entry::Directory);
        assert_content!(map.0.get_mut(OsStr::new("nest/a.txt")).unwrap(), b"a");
        assert_content!(map.0.get_mut(OsStr::new("nest/b.txt")).unwrap(), b"b");
        assert_content!(map.0.get_mut(OsStr::new("o.txt")).unwrap(), b"o");
    }
}
