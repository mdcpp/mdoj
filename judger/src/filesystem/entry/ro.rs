//! collection of entry
//!
//! In tar file, structure is like this:
//! | type | content | ...
//!
//! And we map each type of content to BTreeMap<PathBuf, Entry>

use std::{
    io::{self, SeekFrom},
    sync::Arc,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt},
    sync::Mutex,
};

use super::{FuseReadTrait, BLOCKSIZE, MAX_READ_BLK};

/// A block in tar file, should be readonly
///
/// Note that [`TarBlock`] behavior like [`tokio::fs::File`],
/// except that it doesn't share the same underlying file session
/// by cloning(Reads, writes, and seeks would **not** affect both
/// [`TarBlock`] instances simultaneously.)
#[derive(Debug)]
pub struct TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin,
{
    file: Arc<Mutex<F>>,
    start: u64,
    size: u32,
    cursor: u32,
}

impl<F> TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    pub fn new(file: Arc<Mutex<F>>, start: u64, size: u32) -> Self {
        log::trace!("new block: start={}, size={}", start, size);
        Self {
            file,
            start,
            size,
            cursor: 0,
        }
    }
    #[inline]
    pub fn get_size(&self) -> u32 {
        self.size
    }
    pub async fn read_all(&self) -> io::Result<Vec<u8>> {
        let mut lock = self.file.lock().await;
        lock.seek(SeekFrom::Start(self.start)).await?;
        let mut buf = vec![0_u8; self.size as usize];
        lock.read_exact(&mut buf).await?;
        Ok(buf)
    }
    #[cfg(test)]
    #[allow(dead_code)]
    fn from_raw(file: F, start: u64, size: u32) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
            start,
            size,
            cursor: 0,
        }
    }
    #[inline]
    fn get_seek_from(&self, offset: u64) -> Option<SeekFrom> {
        if self.cursor > self.size {
            None
        } else {
            Some(SeekFrom::Start(self.start + offset + self.cursor as u64))
        }
    }
}

impl<F> FuseReadTrait for TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    async fn read(&mut self, offset: u64, size: u32) -> io::Result<bytes::Bytes> {
        let size = size.min(self.size - self.cursor) as usize;
        let size = size.min(BLOCKSIZE * MAX_READ_BLK);

        let mut lock = self.file.lock().await;
        let seek_from = self.get_seek_from(offset).ok_or(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "tar block out of bound",
        ))?;
        lock.seek(seek_from).await?;

        let mut buf = vec![0_u8; size];

        if let Err(err) = lock.read_exact(&mut buf).await {
            log::warn!("tarball change at runtime, result in error: {}", err);
        }

        Ok(bytes::Bytes::from(buf))
    }
}

impl<F> PartialEq for TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin,
{
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.file, &other.file)
            && self.start == other.start
            && self.size == other.size
            && self.cursor == other.cursor
    }
}

impl<F> Clone for TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin,
{
    fn clone(&self) -> Self {
        Self {
            file: self.file.clone(),
            start: self.start,
            size: self.size,
            cursor: 0,
        }
    }
}
