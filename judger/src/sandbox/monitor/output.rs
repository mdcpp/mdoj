use std::{pin::Pin, task::*};

use futures_core::Future;
use tokio::io::*;

use crate::sandbox::monitor::MonitorKind;

pub type Output = u64;

/// A [`Future`] that never resolves.
struct Never;
impl Future for Never {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct Monitor<I> {
    buffer: Vec<u8>,
    reader: Option<BufReader<Take<I>>>,
    ole: bool,
}

/// Monitor the output of the process
///
impl<I: AsyncRead + Unpin> Monitor<I> {
    fn inner_new(limit: Output, stdin: I) -> Self {
        Self {
            buffer: Vec::with_capacity(limit as usize / 4),
            reader: Some(BufReader::new(stdin.take(limit))),
            ole: false,
        }
    }
    async fn inner_wait_exhaust(&mut self) -> Result<MonitorKind> {
        if let Some(mut reader) = self.reader.take() {
            reader.read_to_end(&mut self.buffer).await?;

            let mut inner_reader: I = reader.into_inner().into_inner();
            if inner_reader.read_u8().await.is_ok() {
                self.ole = true;
                return Ok(MonitorKind::Output);
            }
        }
        Never.await;
        unreachable!("Never return")
    }
    pub fn take_buffer(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buffer)
    }
}

impl<P: AsyncRead + Unpin> Monitor<P> {
    pub fn new(limit: Output, stdout: P) -> Self {
        Self::inner_new(limit, stdout)
    }
}

impl<I: AsyncRead + Unpin> super::Monitor for Monitor<I> {
    type Resource = Output;

    async fn wait_exhaust(&mut self) -> MonitorKind {
        self.inner_wait_exhaust().await.unwrap()
    }
    fn poll_exhaust(&mut self) -> Option<MonitorKind> {
        if !self.ole {
            return None;
        }
        Some(MonitorKind::Output)
    }
    /// This method may report incorrect value due to tokio's guarantee of cancellation safety
    async fn stat(self) -> Self::Resource {
        self.buffer.len() as Output
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn monitor_output_limit() {
        let (mut stdin, stdout) = tokio::io::duplex(1024);
        let mut monitor = Monitor::inner_new(9, stdout);
        stdin.write_all(b"1234567890").await.unwrap();
        assert_eq!(
            MonitorKind::Output,
            monitor.inner_wait_exhaust().await.unwrap(),
        );
    }
}
