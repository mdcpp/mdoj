// todo!(): add resource limit

use std::{collections::VecDeque, sync::Mutex};

use tokio::sync::oneshot;

pub struct ResourceCounter(Mutex<ResourceCounterInner>);

impl ResourceCounter {
    pub fn new(memory: i64) -> Self {
        Self(Mutex::new(ResourceCounterInner {
            memory,
            queue: VecDeque::new(),
        }))
    }
    pub async fn allocate(&self, memory: i64) -> ResourceGuard {
        let rx={
            let mut self_lock = self.0.lock().unwrap();

            let (tx, rx) = oneshot::channel();

            self_lock.queue.push_back((memory, tx));

            self.deallocate(memory);

            rx
        };

        rx.await.unwrap();

        ResourceGuard {
            memory,
            counter: &self,
        }
    }
    fn deallocate(&self, de_memory: i64) {
        let mut self_lock = &mut *self.0.lock().unwrap();

        self_lock.memory += de_memory;
        if let Some((memory, channel)) = self_lock.queue.front() {
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
    queue: VecDeque<(i64, oneshot::Sender<()>)>,
}

pub struct ResourceGuard<'a> {
    memory: i64,
    counter: &'a ResourceCounter,
}

impl<'a> Drop for ResourceGuard<'a> {
    fn drop(&mut self) {
        self.counter.deallocate(self.memory);
    }
}
