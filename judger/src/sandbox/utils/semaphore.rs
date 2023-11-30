// todo!(): add resource limit

use std::{
    collections::BTreeSet,
    sync::{atomic, Arc},
};

use spin::mutex::Mutex;
use tokio::sync::oneshot;

use crate::init::config::CONFIG;

use super::super::Error;

static MEMID: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

pub struct MemoryStatistic {
    pub available_mem: u64,
    pub memory: u64,
    pub tasks: u64,
}

/// A Semaphore for large buffer accounting
/// because tokio::sync::Semaphore default to u32 for inner type
#[derive(Clone)]
pub struct MemorySemaphore(Arc<Mutex<MemorySemaphoreInner>>);

impl MemorySemaphore {
    #[tracing::instrument]
    pub fn new(memory: u64) -> Self {
        Self(Arc::new(Mutex::new(MemorySemaphoreInner {
            memory,
            all_mem: memory,
            queue: BTreeSet::new(),
            tasks: 0,
        })))
    }
    pub fn usage(&self) -> MemoryStatistic {
        let self_ = self.0.lock();
        MemoryStatistic {
            available_mem: self_.memory,
            memory: self_.all_mem,
            tasks: self_.tasks,
        }
    }
    #[tracing::instrument(skip(self),level = tracing::Level::TRACE)]
    pub async fn allocate(&self, memory: u64) -> Result<MemoryPermit, Error> {
        log::trace!("preserve {}B memory", memory);
        let config = CONFIG.get().unwrap();

        if memory > config.platform.available_memory {
            return Err(Error::ImpossibleResource);
        }

        let rx: oneshot::Receiver<()> = {
            let mut self_lock = self.0.lock();

            let (tx, rx) = oneshot::channel();

            self_lock.queue.insert(MemDemand {
                memory,
                tx,
                id: MEMID.fetch_add(1, atomic::Ordering::SeqCst),
            });
            drop(self_lock);

            self.deallocate(0);

            rx
        };

        rx.await.unwrap();

        log::trace!("get {}B memory", memory);

        Ok(MemoryPermit::new(self, memory))
    }
    #[tracing::instrument(skip(self),level = tracing::Level::TRACE)]
    fn deallocate(&self, released_memory: u64) {
        let self_ = &mut *self.0.lock();

        self_.memory += released_memory;
        while let Some(demand) = self_.queue.last() {
            if demand.memory <= self_.memory {
                self_.memory -= demand.memory;
                let channel = self_.queue.pop_last().unwrap().tx;
                channel.send(()).unwrap();
            } else {
                break;
            }
        }
    }
}

pub struct MemorySemaphoreInner {
    memory: u64,
    all_mem: u64,
    queue: BTreeSet<MemDemand>,
    tasks: u64,
}

struct MemDemand {
    memory: u64,
    tx: oneshot::Sender<()>,
    id: usize,
}

impl Ord for MemDemand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.memory, &self.id).cmp(&(other.memory, &other.id))
    }
}

impl PartialOrd for MemDemand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for MemDemand {}
impl PartialEq for MemDemand {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct MemoryPermit {
    memory: u64,
    counter: MemorySemaphore,
}

impl MemoryPermit {
    fn new(counter: &MemorySemaphore, memory: u64) -> Self {
        counter.0.lock().tasks += 1;
        Self {
            memory,
            counter: counter.clone(),
        }
    }
    pub fn downgrade(mut self, target: u64) -> Self {
        self.counter.deallocate(self.memory - target);
        self.memory = target;
        self
    }
}

impl Drop for MemoryPermit {
    fn drop(&mut self) {
        {
            self.counter.0.lock().tasks -= 1;
        }
        self.counter.deallocate(self.memory);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_semaphore() {
        crate::init::new().await;
        let semaphore = MemorySemaphore::new(100);
        let permit = semaphore.allocate(10).await.unwrap();
        assert_eq!(semaphore.usage().available_mem, 90);
        drop(permit);
        assert_eq!(semaphore.usage().available_mem, 100);
        let permit = semaphore.allocate(100).await.unwrap();
        assert_eq!(semaphore.usage().available_mem, 0);
        drop(permit);
    }
}
