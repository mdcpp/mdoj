use crate::filesystem::macro_::{chain_poll, report_poll};
use std::future::Future;
use std::io;
use std::{
    io::SeekFrom,
    ops::{Deref, DerefMut},
    pin::{pin, Pin},
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncSeek, AsyncWrite},
    sync::{Mutex, OwnedMutexGuard},
};

const MEMBLOCK_BLOCKSIZE: usize = 4096;

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
    F: AsyncRead + AsyncSeek + Unpin,
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
    F: AsyncRead + AsyncSeek + Unpin,
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

    use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader};

    use super::*;
    #[tokio::test]
    async fn tar_normal_read() {
        let underlying = BufReader::new(Cursor::new(b"111hello world111"));
        let mut block = TarBlock::from_raw(underlying, 3, 11);

        let mut buf = [0_u8; 11];
        block.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf, *b"hello world");
    }
    #[tokio::test]
    async fn tar_end_of_file_read() {
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
    async fn tar_multi_sequential_read() {
        let underlying = BufReader::new(Cursor::new(b"111hello world111"));
        let mut block = TarBlock::from_raw(underlying, 3, 11);

        for c in b"hello world" {
            assert_eq!(block.read_u8().await.unwrap(), *c);
        }
    }
    #[tokio::test]
    async fn tar_multi_reader_read() {
        let underlying = BufReader::new(Cursor::new(b"111hello world111"));
        let underlying = Arc::new(Mutex::new(underlying));
        let block = TarBlock::new(underlying, 3, 11);

        for _ in 0..3000 {
            let mut block = block.clone();
            tokio::spawn(async move {
                let mut buf = [0_u8; 11];
                block.read_exact(&mut buf).await.unwrap();
                assert_eq!(buf, *b"hello world");
            });
        }
    }
}
