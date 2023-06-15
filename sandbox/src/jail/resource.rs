// todo!(): add resource limit

use std::{collections::VecDeque, sync::Mutex};

use tokio::{sync::oneshot, time};

use crate::init::config::CONFIG;

use super::Error;

pub struct ResourceUsage {
    pub available_mem: i64,
    pub all_available_mem: i64,
    pub tasks: u64,
}

pub struct ResourceCounter(Mutex<ResourceCounterInner>);

impl ResourceCounter {
    pub fn new(memory: i64) -> Self {
        Self(Mutex::new(ResourceCounterInner {
            memory,
            all_mem: memory,
            queue: VecDeque::new(),
            tasks: 0,
        }))
    }
    pub fn usage(&self) -> ResourceUsage {
        let self_lock = self.0.lock().unwrap();
        ResourceUsage {
            available_mem: self_lock.memory,
            all_available_mem: self_lock.all_mem,
            tasks: self_lock.tasks,
        }
    }
    pub async fn allocate(&self, memory: i64) -> Result<ResourceGuard, Error> {
        log::trace!("preserve {}B memory", memory);
        let config = CONFIG.get().unwrap();

        if memory > config.platform.available_memory {
            return Err(Error::InsufficientResource);
        }

        let rx = {
            let mut self_lock = self.0.lock().unwrap();

            let (tx, rx) = oneshot::channel();

            self_lock.queue.push_back((memory, tx));

            self.deallocate(memory);

            rx
        };

        rx.await.unwrap();

        Ok(ResourceGuard::new(self, memory))
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

pub struct ResourceCounterInner {
    memory: i64,
    all_mem: i64,
    queue: VecDeque<(i64, oneshot::Sender<()>)>,
    tasks: u64,
}

pub struct ResourceGuard<'a> {
    mem: i64,
    counter: &'a ResourceCounter,
}

impl<'a> ResourceGuard<'a> {
    fn new(counter: &'a ResourceCounter, memory: i64) -> Self {
        counter.0.lock().unwrap().tasks += 1;
        Self {
            mem: memory,
            counter,
        }
    }
}

impl<'a> Drop for ResourceGuard<'a> {
    fn drop(&mut self) {
        {
            self.counter.0.lock().unwrap().tasks -= 1;
        }
        self.counter.deallocate(self.mem);
    }
}
