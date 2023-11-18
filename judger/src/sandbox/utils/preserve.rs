// todo!(): add resource limit

use std::{
    collections::BTreeSet,
    sync::{atomic, Arc},
};

use spin::mutex::Mutex;
use tokio::sync::oneshot;

use crate::init::config::CONFIG;

use super::super::Error;

const MEMID: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

pub struct MemoryStatistic {
    pub available_mem: i64,
    pub max_mem: i64,
    pub tasks: u64,
}

/// A Semaphore for memory(used instead bc of tokio::sync::Semaphore default to u32 for inner type)
#[derive(Clone)]
pub struct MemorySemaphore(Arc<Mutex<MemoryCounterInner>>);

impl MemorySemaphore {
    pub fn new(memory: i64) -> Self {
        Self(Arc::new(Mutex::new(MemoryCounterInner {
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
            max_mem: self_.all_mem,
            tasks: self_.tasks,
        }
    }
    pub async fn allocate(&self, memory: i64) -> Result<MemoryPermit, Error> {
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
    fn deallocate(&self, de_memory: i64) {
        let self_ = &mut *self.0.lock();

        self_.memory += de_memory;
        while let Some(demand) = self_.queue.last() {
            if demand.memory < self_.memory {
                self_.memory -= demand.memory;
                let channel = self_.queue.pop_last().unwrap().tx;
                channel.send(()).unwrap();
            } else {
                break;
            }
        }
    }
}

pub struct MemoryCounterInner {
    memory: i64,
    all_mem: i64,
    queue: BTreeSet<MemDemand>,
    tasks: u64,
}

struct MemDemand {
    memory: i64,
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
        self.memory.partial_cmp(&other.memory)
    }
}

impl Eq for MemDemand {}
impl PartialEq for MemDemand {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct MemoryPermit {
    memory: i64,
    counter: MemorySemaphore,
}

impl MemoryPermit {
    fn new(counter: &MemorySemaphore, memory: i64) -> Self {
        counter.0.lock().tasks += 1;
        Self {
            memory,
            counter: counter.clone(),
        }
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
mod test{
    use super::*;
    #[tokio::test]
    async fn basic_semaphore(){
        crate::init::new().await;
        let semaphore=MemorySemaphore::new(10000);
        drop(semaphore.allocate(9999).await.unwrap());
    }
}