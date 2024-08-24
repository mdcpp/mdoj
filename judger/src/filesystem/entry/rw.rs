use spin::Mutex;
use std::{io, ops::Deref, sync::Arc};

use super::{FuseFlushTrait, FuseReadTrait, FuseWriteTrait};

/// A block in memory
///
/// Note that [`MemBlock`] behavior like [`tokio::fs::File`],
/// except that it doesn't share the same underlying file session
/// by cloning(Reads, writes, and seeks would **not** affect both
/// [`MemBlock`] instances simultaneously.)
#[derive(Default, Debug)]
pub struct MemBlock {
    data: Arc<Mutex<Vec<u8>>>,
    /// when file is in read mode, cursor is the position of the next byte to read
    ///
    /// when file is in write mode, cursor at of the write buffer(append)
    cursor: usize,
    write_buffer: Vec<u8>,
}

impl MemBlock {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: Arc::new(Mutex::new(data)),
            cursor: 0,
            write_buffer: Vec::new(),
        }
    }
    pub fn set_append(&mut self) {
        self.cursor = self.data.lock().len();
    }
    pub fn get_size(&self) -> u64 {
        self.data.lock().len() as u64
    }
}

impl FuseReadTrait for MemBlock {
    async fn read(&mut self, offset: u64, size: u32) -> io::Result<bytes::Bytes> {
        let locked = self.data.lock();
        if locked.len() < offset as usize {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "mem block out of bound",
            ));
        }
        let offset = offset as usize;
        let slice = &locked.deref()[offset..(offset + size as usize).min(locked.len())];
        Ok(bytes::Bytes::copy_from_slice(slice))
    }
}
impl FuseWriteTrait for MemBlock {
    async fn write(&mut self, offset: u64, data: &[u8]) -> io::Result<u32> {
        // FIXME: file hole may cause OOM
        let mut locked = self.data.lock();
        if self.cursor > locked.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "mem block out of bound",
            ));
        }
        let new_size = self.cursor + offset as usize + data.len();
        if locked.len() < new_size {
            locked.resize(new_size, 0);
        }
        for i in 0..data.len() {
            locked[self.cursor + offset as usize + i] = data[i];
        }
        Ok(data.len() as u32)
    }
}

impl FuseFlushTrait for MemBlock {
    async fn flush(&mut self) -> io::Result<()> {
        let mut locked = self.data.lock();
        locked.extend_from_slice(&self.write_buffer);
        self.write_buffer.clear();
        Ok(())
    }
}

impl Clone for MemBlock {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            cursor: 0,
            write_buffer: self.write_buffer.clone(),
        }
    }
}
