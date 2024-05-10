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

use super::FuseReadTrait;

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
    size: u32,
    cursor: u32,
}

impl<F> TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    pub fn new(file: Arc<Mutex<F>>, start: u64, size: u32) -> Self {
        log::info!("new block: start={}, size={}", start, size);
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
    // pub async fn read_all(&self) -> std::io::Result<Vec<u8>> {
    //     // let mut buf = Vec::with_capacity(self.size as usize);
    //     // let mut block = self.clone();
    //     // block.seek(SeekFrom::Start(0)).await?;
    //     // block.read_to_end(&mut buf).await?;
    //     // Ok(buf)
    //     todo!()
    // }
    #[cfg(test)]
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
            Some(SeekFrom::Start(self.start + offset + (self.cursor) as u64))
        }
    }
    #[inline]
    fn get_remain(&self) -> u32 {
        self.size - self.cursor
    }
}

impl<F> FuseReadTrait for TarBlock<F>
where
    F: AsyncRead + AsyncSeek + Unpin + 'static,
{
    async fn read(&mut self, offset: u64, size: u32) -> std::io::Result<bytes::Bytes> {
        let size = size as usize;
        let mut lock = self.file.lock().await;
        let seek_from = self.get_seek_from(offset).ok_or(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "tar block out of bound",
        ))?;
        lock.seek(seek_from).await?;

        let mut buf = vec![0_u8; size];

        let mut readed_byte = 0;
        while readed_byte < size {
            match lock.read(&mut buf).await {
                Err(err) if readed_byte == 0 => return Err(err),
                Ok(0) | Err(_) => break,
                Ok(x) => readed_byte += x,
            };
        }
        readed_byte = readed_byte.min(size);
        self.cursor += readed_byte as u32;

        buf.resize(readed_byte, 0_u8);
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

// #[cfg(test)]
// mod test {
//     use std::io::Cursor;

//     use tokio::{fs::File, io::BufReader};

//     use super::*;
//     #[tokio::test]
//     async fn file_io() {
//         let file = File::open("test/single_file.tar").await.unwrap();
//         let mut block = TarBlock::new(Arc::new(Mutex::new(file)), 512, 11);
//         let mut buf = [0_u8; 11];
//         block.read_exact(&mut buf).await.unwrap();
//         assert_eq!(buf, *b"hello world");
//     }
//     #[tokio::test]
//     async fn normal_read() {
//         let underlying = BufReader::new(Cursor::new(b"111hello world111"));
//         let mut block = TarBlock::from_raw(underlying, 3, 11);

//         let mut buf = [0_u8; 11];
//         block.read_exact(&mut buf).await.unwrap();

//         assert_eq!(buf, *b"hello world");
//     }
//     #[tokio::test]
//     async fn end_of_file_read() {
//         let underlying = BufReader::new(Cursor::new(b"111hello world"));
//         let mut block = TarBlock::from_raw(underlying, 3, 11);

//         let mut buf = [0_u8; 11];
//         block.read_exact(&mut buf).await.unwrap();

//         assert_eq!(
//             block.read_u8().await.unwrap_err().kind(),
//             io::ErrorKind::UnexpectedEof
//         );
//     }
//     #[tokio::test]
//     async fn multi_sequential_read() {
//         let underlying = BufReader::new(Cursor::new(b"111hello world111"));
//         let mut block = TarBlock::from_raw(underlying, 3, 11);

//         for c in b"hello world" {
//             assert_eq!(block.read_u8().await.unwrap(), *c);
//         }
//     }
//     #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
//     async fn multi_reader_read() {
//         let underlying = BufReader::new(Cursor::new(b"111hello world111"));
//         let underlying = Arc::new(Mutex::new(underlying));
//         let block = TarBlock::new(underlying, 3, 11);

//         for _ in 0..30 {
//             let mut block = block.clone();
//             tokio::spawn(async move {
//                 for _ in 0..400 {
//                     let mut buf = [0_u8; 11];
//                     block.read_exact(&mut buf).await.unwrap();
//                     assert_eq!(buf, *b"hello world");
//                 }
//             });
//         }
//     }
// }
