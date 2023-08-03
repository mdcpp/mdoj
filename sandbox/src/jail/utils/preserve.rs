// todo!(): add resource limit

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use tokio::sync::oneshot;

use crate::init::config::CONFIG;

use super::super::Error;

pub struct MemoryStatistic {
    pub available_mem: i64,
    pub all_available_mem: i64,
    pub tasks: u64,
}

#[derive(Clone)]
pub struct MemoryCounter(Arc<Mutex<MemoryCounterInner>>);

impl MemoryCounter {
    pub fn new(memory: i64) -> Self {
        Self(Arc::new(Mutex::new(MemoryCounterInner {
            memory,
            all_mem: memory,
            queue: VecDeque::new(),
            tasks: 0,
        })))
    }
    pub fn usage(&self) -> MemoryStatistic {
        let self_lock = self.0.lock().unwrap();
        MemoryStatistic {
            available_mem: self_lock.memory,
            all_available_mem: self_lock.all_mem,
            tasks: self_lock.tasks,
        }
    }
    pub async fn allocate(&self, memory: i64) -> Result<MemoryHolder, Error> {
        log::trace!("preserve {}B memory", memory);
        let config = CONFIG.get().unwrap();

        if memory > config.platform.available_memory {
            return Err(Error::ImpossibleResource);
        }

        let rx: oneshot::Receiver<()> = {
            let mut self_lock = self.0.lock().unwrap();

            let (tx, rx) = oneshot::channel();

            self_lock.queue.push_back((memory, tx));

            drop(self_lock);

            self.deallocate(memory);

            rx
        };

        rx.await.unwrap();

        log::trace!("get {}B memory", memory);

        Ok(MemoryHolder::new(self, memory))
    }
    fn deallocate(&self, de_memory: i64) {
        let mut self_lock = &mut *self.0.lock().unwrap();

        self_lock.memory += de_memory;
        if let Some((memory, _)) = self_lock.queue.front() {
            if memory < &self_lock.memory {
                self_lock.memory -= memory;
                let (_, channel) = self_lock.queue.pop_front().unwrap();
                channel.send(()).unwrap();
            }
        }
    }
}

pub struct MemoryCounterInner {
    memory: i64,
    all_mem: i64,
    queue: VecDeque<(i64, oneshot::Sender<()>)>,
    tasks: u64,
}

pub struct MemoryHolder {
    mem: i64,
    counter: MemoryCounter,
}

impl MemoryHolder {
    fn new(counter: &MemoryCounter, memory: i64) -> Self {
        counter.0.lock().unwrap().tasks += 1;
        Self {
            mem: memory,
            counter: counter.clone(),
        }
    }
}

impl Drop for MemoryHolder {
    fn drop(&mut self) {
        {
            self.counter.0.lock().unwrap().tasks -= 1;
        }
        self.counter.deallocate(self.mem);
    }
}
