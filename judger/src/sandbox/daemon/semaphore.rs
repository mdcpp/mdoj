use std::{
    fmt::Debug,
    sync::{
        atomic::{self, Ordering},
        Arc,
    },
};

use crate::error::Error as CrateError;
use spin::Mutex;
use tokio::sync::oneshot::*;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("Max wait reached")]
    MaxWaitReached,
    #[error("Impossible to get the permit")]
    ImpossibleResourceCondition,
}

impl From<Error> for CrateError {
    fn from(value: Error) -> CrateError {
        match value {
            Error::MaxWaitReached => CrateError::QueueFull,
            Error::ImpossibleResourceCondition => CrateError::LowMemory,
        }
    }
}

struct SemaphoreInner {
    permits: atomic::AtomicU64,
    all_permits: u64,
    max_wait: usize,
    waiters: Mutex<Vec<(u64, Option<Sender<()>>)>>,
}

#[derive(Clone)]
pub struct Semaphore(Arc<SemaphoreInner>);

impl Semaphore {
    /// Create a new asynchronous semaphore with the given number of permits.
    ///
    /// asynchronous semaphore is a synchronization primitive that limits the number of concurrent,
    /// instead of blocking the thread, yeild to scheduler and wait for the permit.
    ///
    /// Note that there is no preemption.
    pub fn new(all_permits: u64, max_wait: usize) -> Self {
        Semaphore(Arc::new(SemaphoreInner {
            permits: atomic::AtomicU64::new(all_permits),
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
    pub async fn get_permit(&self, permit: u64) -> Result<Permit, Error> {
        // FIXME: return Result to differentiate between max_wait_reached and impossible_resource_condition
        if permit > self.0.all_permits {
            return Err(Error::ImpossibleResourceCondition);
        }
        let (tx, rx) = channel::<()>();
        {
            let mut waiter = self.0.waiters.lock();
            if waiter.len() >= self.0.max_wait {
                return Err(Error::MaxWaitReached);
            }
            waiter.push((permit, Some(tx)));
        }

        self.try_wake();

        rx.await.ok().expect("Channel closed");

        Ok(Permit {
            semaphore: self.clone(),
            permit,
        })
    }
    fn release(&self, permit: u64) {
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

pub(super) struct Permit {
    semaphore: Semaphore,
    permit: u64,
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.semaphore.release(self.permit);
    }
}

impl Debug for Permit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Permit")
            .field("permit", &self.permit)
            .finish()
    }
}

impl PartialEq for Permit {
    fn eq(&self, other: &Self) -> bool {
        self.permit == other.permit
    }
}

#[cfg(test)]
mod test {
    use tokio::time;

    use super::*;
    #[tokio::test]
    /// test [`Semaphore::get_permit`] return [`Err(Error::ImpossibleResourceCondition)`] when max_wait is reached
    async fn get_permit_max() {
        let semaphore = Semaphore::new(1024, 1024);
        assert!(semaphore.get_permit(1024).await.is_ok());
        assert_eq!(
            Err(Error::ImpossibleResourceCondition),
            semaphore.get_permit(1025).await
        );
    }
    #[tokio::test]
    /// test [`Semaphore::get_permit`] to ensure permit is distributed in order
    /// (First come first serve, no matter amount of permit requested)
    async fn get_permit_unorder() {
        let semaphore = Semaphore::new(1024, 1024);
        let permit = semaphore.get_permit(1).await.unwrap();
        let permit1 = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            semaphore.get_permit(1024).await
        });
        drop(permit);
        assert!(permit1.await.unwrap().is_ok());
    }
    #[tokio::test]
    /// test [`Semaphore::get_permit`] return [`Err(Error::MaxWaitReached)`] when max_wait is reached
    async fn get_permit_max_wait() {
        let semaphore = Semaphore::new(1024, 1);
        let permit = semaphore.get_permit(1).await.unwrap();

        let semaphore1 = semaphore.clone();

        let _ = tokio::spawn(async move {
            semaphore.get_permit(1024).await.unwrap();
        });

        time::sleep(time::Duration::from_millis(4)).await;
        assert_eq!(Err(Error::MaxWaitReached), semaphore1.get_permit(1).await);

        drop(permit);
    }
}
