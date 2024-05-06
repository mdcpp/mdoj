//! collection of entry
//!
//! In tar file, structure is like this:
//! | type | content | ...
//!
//! And we map each type of content to BTreeMap<PathBuf, Entry>

use std::ffi::OsString;

use crate::filesystem::{
    macro_::{chain_poll, report_poll},
    FuseError,
};
use bytes::Bytes;
use fuse3::FileType;
use std::{
    future::Future,
    io::{self, SeekFrom},
    ops::DerefMut,
    pin::{pin, Pin},
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncSeek},
    sync::{Mutex, OwnedMutexGuard},
};

use super::wrapper::FuseRead;

/// Entry from tar file, should be readonly
#[derive(Debug, Default)]
pub enum Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    SymLink(OsString),
    HardLink(u64),
    #[default]
    Directory,
    File(TarBlock<F>),
}

impl<F> Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    #[inline]
    pub fn new_dir() -> Self {
        Self::default()
    }
    #[inline]
    pub fn new_file(block: TarBlock<F>) -> Self {
        Self::File(block)
    }
    #[inline]
    pub fn new_symlink(target: OsString) -> Self {
        Self::SymLink(target)
    }
    #[inline]
    pub fn kind(&self) -> FileType {
        match self {
            Self::SymLink(_) => FileType::Symlink,
            Self::HardLink(_) => FileType::RegularFile,
            Self::Directory => FileType::Directory,
            Self::File(_) => FileType::RegularFile,
        }
    }
    pub async fn read(&mut self, offset: u64, size: u32) -> Result<Bytes, FuseError> {
        // FIXME: follow symlink
        if let Self::File(block) = self {
            return FuseRead(block).read(offset, size).await;
        }
        Err(FuseError::IsDir)
    }
    pub async fn write(&mut self, offset: u64, data: &[u8]) -> Result<u32, FuseError> {
        Err(FuseError::Unimplemented)
    }
}

impl<F> Clone for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::SymLink(arg0) => Self::SymLink(arg0.clone()),
            Self::HardLink(arg0) => Self::HardLink(arg0.clone()),
            Self::Directory => Self::Directory,
            Self::File(arg0) => Self::File(arg0.clone()),
        }
    }
}

impl<F> PartialEq for Entry<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SymLink(l0), Self::SymLink(r0)) => l0 == r0,
            (Self::File(l0), Self::File(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[derive(Default, Debug)]
enum TarStage<F> {
    Reading(OwnedMutexGuard<F>),
    Seeking(OwnedMutexGuard<F>),
    #[default]
    Done,
}

impl<F> TarStage<F> {
    fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

/// A block in tar file, should be readonly
///
/// Note that [`TarBlock`] behavior like [`tokio::fs::File`],
/// except that it dones't shares the same underlying file session
/// by cloning(Reads, writes, and seeks would **not** affect both
/// [`TarBlock`] instances simultaneously.)
#[derive(Debug)]
pub struct TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin,
{
    file: Arc<Mutex<F>>,
    start: u64,
    size: u64,
    cursor: u64,
    stage: TarStage<F>,
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
            cursor: self.cursor,
            stage: TarStage::Done,
        }
    }
}

impl<F> TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    pub fn new(file: Arc<Mutex<F>>, start: u64, size: u64) -> Self {
        Self {
            file,
            start,
            size,
            cursor: 0,
            stage: TarStage::Done,
        }
    }
    #[cfg(test)]
    fn from_raw(file: F, start: u64, size: u64) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
            start,
            size,
            cursor: 0,
            stage: TarStage::Done,
        }
    }
    #[inline]
    fn get_seek_from(&self) -> SeekFrom {
        SeekFrom::Start(self.start + self.cursor)
    }
    #[inline]
    fn check_bound(&self) -> bool {
        self.cursor > self.size
    }
    #[inline]
    fn get_remain(&self) -> u64 {
        self.size - self.cursor
    }
}

impl<F> AsyncRead for TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.check_bound() {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "tar block out of bound",
            )));
        }
        let original_size = buf.filled().len();
        match self.stage.take() {
            TarStage::Reading(mut locked) => {
                report_poll!(chain_poll!(pin!(locked.deref_mut()).poll_read(cx, buf)));
                let read_byte = (buf.filled().len() - original_size) as u64;
                match read_byte > self.get_remain() {
                    true => {
                        buf.set_filled(original_size + self.get_remain() as usize);
                        self.cursor += self.get_remain();
                    }
                    false => self.cursor += read_byte,
                };
                return Poll::Ready(Ok(()));
            }
            TarStage::Seeking(mut locked) => {
                let result = chain_poll!(pin!(locked.deref_mut()).poll_complete(cx));
                let read_byte = report_poll!(result);
                self.as_mut().stage = TarStage::Reading(locked);
                self.as_mut().cursor = read_byte - self.start;
                cx.waker().wake_by_ref();
            }
            TarStage::Done => {
                let mut locked = chain_poll!(pin!(self.file.clone().lock_owned()).poll(cx));
                if let Err(err) = pin!(locked.deref_mut()).start_seek(self.get_seek_from()) {
                    return Poll::Ready(Err(err));
                }
                self.as_mut().stage = TarStage::Seeking(locked);
                cx.waker().wake_by_ref();
            }
        }
        Poll::Pending
    }
}

impl<F> AsyncSeek for TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    fn start_seek(self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        let self_ = self.get_mut();
        self_.cursor = match position {
            io::SeekFrom::Start(x) => x,
            io::SeekFrom::End(x) => (self_.size as i64 + x).try_into().unwrap_or_default(),
            io::SeekFrom::Current(x) => (self_.cursor as i64 + x).try_into().unwrap_or_default(),
        };
        if self_.check_bound() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "tar block out of bound",
            ));
        }
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Poll::Ready(Ok(self.cursor))
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use tokio::io::{AsyncReadExt, BufReader};

    use super::*;
    #[tokio::test]
    async fn normal_read() {
        let underlying = BufReader::new(Cursor::new(b"111hello world111"));
        let mut block = TarBlock::from_raw(underlying, 3, 11);

        let mut buf = [0_u8; 11];
        block.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf, *b"hello world");
    }
    #[tokio::test]
    async fn end_of_file_read() {
        let underlying = BufReader::new(Cursor::new(b"111hello world"));
        let mut block = TarBlock::from_raw(underlying, 3, 11);

        let mut buf = [0_u8; 11];
        block.read_exact(&mut buf).await.unwrap();

        assert_eq!(
            block.read_u8().await.unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }
    #[tokio::test]
    async fn multi_sequential_read() {
        let underlying = BufReader::new(Cursor::new(b"111hello world111"));
        let mut block = TarBlock::from_raw(underlying, 3, 11);

        for c in b"hello world" {
            assert_eq!(block.read_u8().await.unwrap(), *c);
        }
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn multi_reader_read() {
        let underlying = BufReader::new(Cursor::new(b"111hello world111"));
        let underlying = Arc::new(Mutex::new(underlying));
        let block = TarBlock::new(underlying, 3, 11);

        for _ in 0..30 {
            let mut block = block.clone();
            tokio::spawn(async move {
                for _ in 0..400 {
                    let mut buf = [0_u8; 11];
                    block.read_exact(&mut buf).await.unwrap();
                    assert_eq!(buf, *b"hello world");
                }
            });
        }
    }
}
