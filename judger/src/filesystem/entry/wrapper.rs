use std::io::SeekFrom;

use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

pub struct FuseRead<'a, W>(pub &'a mut W)
where
    W: AsyncRead + AsyncSeek + Clone + Unpin;

impl<'a, W> FuseRead<'a, W>
where
    W: AsyncRead + AsyncSeek + Clone + Unpin,
{
    pub async fn read(&mut self, offset: u64, size: u32) -> std::io::Result<Bytes> {
        let mut buf = Vec::with_capacity(size as usize);
        self.0.seek(SeekFrom::Start(offset)).await?;

        self.0
            .clone()
            .take(size as u64)
            .read_to_end(&mut buf)
            .await?;

        self.0.seek(SeekFrom::Current(buf.len() as i64)).await?;

        Ok(buf.try_into().unwrap())
    }
}

pub struct FuseWrite<'a, W>(pub &'a mut W)
where
    W: AsyncWrite + AsyncSeek + Clone + Unpin;

impl<'a, W> FuseWrite<'a, W>
where
    W: AsyncWrite + AsyncSeek + Clone + Unpin,
{
    pub async fn write(&mut self, offset: u64, data: &[u8]) -> std::io::Result<u32> {
        assert!(data.len() < (u32::MAX - 1) as usize);
        self.0.seek(SeekFrom::Start(offset)).await?;

        self.0.write_all(data).await?;
        Ok(data.len() as u32)
    }
}
