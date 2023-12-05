use spin::mutex::Mutex;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::{collections::HashMap, hash::Hash, sync::Arc};
use tokio::sync::broadcast::*;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

pub struct PubGuard<M, I>
where
    M: Clone + Send + 'static,
    I: Eq + Clone + Hash + Send + 'static,
{
    pubsub: Arc<PubSub<M, I>>,
    id: I,
    tx: Sender<M>,
}

impl<M, I> Deref for PubGuard<M, I>
where
    M: Clone + Send + 'static,
    I: Eq + Clone + Hash + Send + 'static,
{
    type Target = Sender<M>;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

impl<M, I> DerefMut for PubGuard<M, I>
where
    M: Clone + Send + 'static,
    I: Eq + Clone + Hash + Send + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tx
    }
}

impl<M: Clone + Send + 'static, I: Eq + Clone + Hash + Send + 'static> Drop for PubGuard<M, I> {
    fn drop(&mut self) {
        self.pubsub.outgoing.lock().remove(&self.id);
    }
}

pub struct PubSub<M, I> {
    outgoing: Mutex<HashMap<I, Receiver<M>>>,
}

impl<M, I> Default for PubSub<M, I> {
    fn default() -> Self {
        PubSub {
            outgoing: Mutex::new(HashMap::new()),
        }
    }
}

impl<M, I> PubSub<M, I>
where
    M: Clone + Send + 'static,
    I: Eq + Clone + Hash + Send + 'static,
{
    pub fn publish(self: &Arc<Self>, id: I) -> PubGuard<M, I> {
        let (tx, rx) = channel(16);
        self.outgoing.lock().insert(id.clone(), rx);
        PubGuard {
            pubsub: self.clone(),
            id,
            tx,
        }
    }
    pub fn subscribe(self: &Arc<Self>, id: &I) -> Option<Pin<Box<dyn Stream<Item = M> + Send>>> {
        self.clone().outgoing.lock().get(id).map(|s| {
            Box::pin(BroadcastStream::new(s.resubscribe()).filter_map(|item| {
                item.map_err(|err| match err {
                    BroadcastStreamRecvError::Lagged(x) => {
                        log::trace!("PubSub: lagged {} messeges", x)
                    }
                })
                .ok()
            })) as Pin<Box<dyn Stream<Item = M> + Send>>
        })
    }
}
