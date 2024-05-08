use crate::filesystem::{
    macro_::{chain_poll, report_poll},
    FuseError,
};
use bytes::Bytes;
use fuse3::FileType;
use std::{
    ffi::OsString,
    future::Future,
    io::{self, SeekFrom},
    ops::Deref,
    pin::{pin, Pin},
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncSeek, AsyncWrite},
    sync::{Mutex, OwnedMutexGuard},
};

use super::wrapper::{FuseRead, FuseWrite};
use super::MEMBLOCK_BLOCKSIZE;

#[derive(Default, Clone, Debug)]
pub enum Entry {
    SymLink(OsString),
    HardLink(u64),
    #[default]
    Directory,
    File(MemBlock),
    Removed,
}

impl Entry {
    pub fn new_dir() -> Self {
        Self::default()
    }
    pub fn new_file() -> Self {
        Self::File(MemBlock::new(Vec::new()))
    }
    pub fn new_data(data: Vec<u8>) -> Self {
        Self::File(MemBlock::new(data))
    }
    pub fn new_symlink(target: OsString) -> Self {
        Self::SymLink(target)
    }
    pub fn new_hardlink(inode: u64) -> Self {
        Self::HardLink(inode)
    }
    pub fn kind(&self) -> FileType {
        match self {
            Self::SymLink(_) => FileType::Symlink,
            Self::HardLink(_) => FileType::RegularFile,
            Self::Directory => FileType::Directory,
            Self::File(_) => FileType::RegularFile,
            Self::Removed => FileType::RegularFile,
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
        if let Self::File(block) = self {
            return FuseWrite(block).write(offset, data).await;
        }
        Err(FuseError::IsDir)
    }
}

#[derive(Debug, Default)]
enum MemStage {
    Seeking(OwnedMutexGuard<Vec<u8>>, SeekFrom),
    SeekStart(SeekFrom),
    Reading(OwnedMutexGuard<Vec<u8>>),
    // Writing(OwnedMutexGuard<Vec<u8>>),
    #[default]
    Done,
}

impl MemStage {
    fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

#[derive(Default, Debug)]
pub struct MemBlock {
    data: Arc<Mutex<Vec<u8>>>,
    cursor: usize,
    stage: MemStage,
    write_buffer: Vec<u8>,
}

impl Clone for MemBlock {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            cursor: self.cursor.clone(),
            stage: MemStage::default(),
            write_buffer: self.write_buffer.clone(),
        }
    }
}

impl MemBlock {
    fn new(data: Vec<u8>) -> Self {
        Self {
            data: Arc::new(Mutex::new(data)),
            cursor: 0,
            stage: MemStage::Done,
            write_buffer: Vec::new(),
        }
    }
}

impl AsyncRead for MemBlock {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.stage.take() {
            MemStage::Reading(locked) => {
                if locked.len() < self.cursor {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "mem block out of bound",
                    )));
                }
                let slice = &locked.deref()[self.cursor
                    ..(self.cursor + MEMBLOCK_BLOCKSIZE.min(buf.remaining())).min(locked.len())];
                self.cursor += slice.len();
                buf.put_slice(slice);
                return Poll::Ready(Ok(()));
            }
            _ => {
                let locked = chain_poll!(pin!(self.data.clone().lock_owned()).poll(cx));
                self.as_mut().stage = MemStage::Reading(locked);
                cx.waker().wake_by_ref();
            }
        }
        Poll::Pending
    }
}

impl AsyncSeek for MemBlock {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        self.stage = MemStage::SeekStart(position);
        Ok(())
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        match self.stage.take() {
            MemStage::SeekStart(position) => {
                let locked = chain_poll!(pin!(self.data.clone().lock_owned()).poll(cx));
                self.stage = MemStage::Seeking(locked, position);
                cx.waker().wake_by_ref();
            }
            MemStage::Seeking(locked, position) => {
                let size = locked.len() as i64;
                let new_position = match position {
                    SeekFrom::Start(x) => x.try_into().unwrap_or_default(),
                    SeekFrom::End(x) => size.saturating_sub(x),
                    SeekFrom::Current(x) => (self.cursor as i64).saturating_add(x),
                };
                if new_position < 0 {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "invalid seek position",
                    )));
                }
                if new_position > size {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "mem block out of bound",
                    )));
                }
                self.cursor = new_position as usize;
                return Poll::Ready(Ok(self.cursor as u64));
            }
            _ => {
                return Poll::Ready(Ok(self.cursor as u64));
            }
        }
        Poll::Pending
    }
}

impl AsyncWrite for MemBlock {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.write_buffer.extend_from_slice(&buf);
        if self.write_buffer.len() >= MEMBLOCK_BLOCKSIZE {
            report_poll!(chain_poll!(self.as_mut().poll_flush(cx)));
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let mut locked = chain_poll!(pin!(self.data.clone().lock_owned()).poll(cx));
        locked.extend_from_slice(&self.write_buffer);
        self.write_buffer.clear();
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        self.as_mut().poll_flush(cx)
    }
}

#[cfg(test)]
mod test {
    use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

    use super::*;
    #[tokio::test]
    async fn normal_read() {
        let data = b"hello world".to_vec();
        let mut block = MemBlock::new(data);
        let mut buf = [0_u8; 11];
        block.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf, *b"hello world");
    }
    #[tokio::test]
    async fn end_of_file_read() {
        let mut block = MemBlock::new(b"1234".to_vec());
        let mut buf = Vec::new();
        block.read_to_end(&mut buf).await.unwrap();

        assert_eq!(&*buf, b"1234");
    }
    #[tokio::test]
    async fn start_seek() {
        let mut block = MemBlock::new(b"111hello world1111".to_vec());
        block.seek(SeekFrom::Start(3)).await.unwrap();

        let mut buf = [0_u8; 11];
        block.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf, *b"hello world");
    }
    #[tokio::test]
    async fn end_seek() {
        let mut block = MemBlock::new(b"111hello world1111".to_vec());
        block.seek(SeekFrom::End(15)).await.unwrap();

        let mut buf = [0_u8; 11];
        block.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf, *b"hello world");
    }
    #[tokio::test]
    async fn rel_seek() {
        let mut block = MemBlock::new(b"111hello world1111".to_vec());
        for _ in 0..3 {
            block.seek(SeekFrom::Current(1)).await.unwrap();
        }

        let mut buf = [0_u8; 11];
        block.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf, *b"hello world");
    }
    #[tokio::test]
    async fn normal_write() {
        let mut block = MemBlock::default();
        block.write_all(b"hello").await.unwrap();
        block.write_all(b" ").await.unwrap();
        block.write_all(b"world").await.unwrap();

        assert!(block.read_u8().await.is_err());

        block.flush().await.unwrap();

        let mut buf = [0_u8; 11];
        block.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf, *b"hello world");
    }
    #[tokio::test]
    async fn multi_read() {
        let block = MemBlock::new(b"hello world".to_vec());

        for _ in 0..3000 {
            let mut block = block.clone();
            tokio::spawn(async move {
                let mut buf = [0_u8; 11];
                block.read_exact(&mut buf).await.unwrap();
                assert_eq!(buf, *b"hello world");
            });
        }
    }
    #[tokio::test]
    #[should_panic]
    async fn test_take_read() {
        let block = MemBlock::new(b"hello world".to_vec());
        let mut buffer = [0; 5];

        // read at most five bytes
        let mut handle = block.take(5);
        handle.read_exact(&mut buffer).await.unwrap();
        assert_eq!(buffer, *b"hello");

        // read the rest
        let mut buffer = [0; 6];
        handle.read_exact(&mut buffer).await.unwrap();
        assert_eq!(buffer, *b" world");
    }
    #[tokio::test]
    async fn test_take_short_read() {
        let block = MemBlock::new(b"hello ".to_vec());
        let mut buffer = Vec::new();

        // read at most five bytes
        let mut handle = block.take(100);
        handle.read_to_end(&mut buffer).await.unwrap();
        assert_eq!(buffer, b"hello ");
    }
}
