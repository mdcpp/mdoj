use std::sync::{
    atomic::{self, Ordering},
    Arc,
};

use spin::Mutex;
use tokio::sync::oneshot::*;

struct SemaphoreInner {
    permits: atomic::AtomicUsize,
    all_permits: usize,
    max_wait: usize,
    waiters: Mutex<Vec<(usize, Option<Sender<()>>)>>,
}

#[derive(Clone)]
struct Semaphore(Arc<SemaphoreInner>);

impl Semaphore {
    /// Create a new asynchronous semaphore with the given number of permits.
    ///
    /// asynchronous semaphore is a synchronization primitive that limits the number of concurrent,
    /// instead of blocking the thread, yeild to scheduler and wait for the permit.
    ///
    /// Note that there is no preemption.
    pub fn new(all_permits: usize, max_wait: usize) -> Self {
        Semaphore(Arc::new(SemaphoreInner {
            permits: atomic::AtomicUsize::new(all_permits),
            all_permits,
            max_wait,
            waiters: Mutex::new(Vec::new()),
        }))
    }
    /// get a permit from semaphore
    ///
    /// It return None if
    /// 1. It's impossible to get the permit even no other task is holding the permit
    /// 2. The number of waiting task is greater than max_wait
    pub async fn get_permit(&self, permit: usize) -> Option<Permit> {
        // FIXME: return Result to differentiate between max_wait_reached and impossible_resource_condition
        if permit > self.0.all_permits {
            return None;
        }
        let (tx, rx) = channel::<()>();
        {
            let mut waiter = self.0.waiters.lock();
            if waiter.len() >= self.0.max_wait {
                return None;
            }
            waiter.push((permit, Some(tx)));
        }

        self.try_wake();

        rx.await.ok()?;

        Some(Permit {
            semaphore: self.clone(),
            permit,
        })
    }
    fn release(&self, permit: usize) {
        self.0.permits.fetch_add(permit, Ordering::Relaxed);
        self.try_wake();
    }
    fn try_wake(&self) {
        let mut waiter = self.0.waiters.lock();
        if let Some((permit, ref mut waker)) = waiter.last_mut() {
            let mut current = self.0.permits.load(Ordering::Acquire);
            loop {
                if current < *permit {
                    return;
                }
                if let Err(x) = self.0.permits.compare_exchange(
                    current,
                    current - *permit,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                ) {
                    current = x;
                } else {
                    break;
                };
            }
            if waker.take().unwrap().send(()).is_err() {
                log::warn!("Semaphore waiter disconnected");
            }
            waiter.pop();
        }
    }
}

pub struct Permit {
    semaphore: Semaphore,
    permit: usize,
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.semaphore.release(self.permit);
    }
}

#[cfg(test)]
mod test {
    use super::Semaphore;
    #[tokio::test]
    /// test max value of permit
    async fn get_permit_max() {
        let semaphore = Semaphore::new(1024, 1024);
        assert!(semaphore.get_permit(1024).await.is_some());
        assert!(semaphore.get_permit(1025).await.is_none());
    }
    #[tokio::test]
    // test getting permit with ordering
    async fn get_permit_unorder() {
        let semaphore = Semaphore::new(1024, 1024);
        let permit = semaphore.get_permit(1).await.unwrap();
        let permit1 = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            semaphore.get_permit(1024).await
        });
        drop(permit);
        assert!(permit1.await.unwrap().is_some());
    }
    #[tokio::test]
    // test `get_permit` return None when max_wait is reached
    async fn get_permit_max_wait() {
        let semaphore = Semaphore::new(1024, 1);
        let semaphore1 = semaphore.clone();
        let _ = semaphore.get_permit(1).await.unwrap();
        let _ = tokio::spawn(async move {
            semaphore.get_permit(1024).await.unwrap();
        });
        dbg!(semaphore1.get_permit(1).await.is_none());
    }
}
