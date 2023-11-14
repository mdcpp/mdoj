use spin::mutex::Mutex;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
// use std::pin::Pin;
// use std::task::Poll;
use std::{collections::HashMap, hash::Hash, sync::Arc};
use tokio::sync::broadcast::*;
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

impl<M, I> PubSub<M, I>
where
    M: Clone + Send + 'static,
    I: Eq + Clone + Hash + Send + 'static,
{
    pub fn stream(
        self: &Arc<Self>,
        mut stream: impl Stream<Item = M> + Unpin + Send + 'static,
        id: I,
    ) {
        let tx = {
            let (tx, rx) = channel(16);
            self.outgoing.lock().insert(id.clone(), rx);
            tx
        };

        let self_ = self.clone();
        tokio::spawn(async move {
            while let Some(messenge) = stream.next().await {
                if tx.send(messenge).is_err() {
                    log::trace!("PubSub: messege")
                }
            }
            self_.outgoing.lock().remove(&id);
        });
    }
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
        self.clone()
            .outgoing
            .lock()
            .get(id)
            .map(|s| BroadcastStream::new(s.resubscribe()))
            .map(|s| {
                Box::pin(s.filter_map(|item| item.ok())) as Pin<Box<dyn Stream<Item = M> + Send>>
            })
    }
}

// pub struct SubStream<M>(BroadcastStream<Option<M>>);

// impl<M> Stream for SubStream<M>
// where
//     M: 'static + Clone + Send,
// {
//     type Item = M;

//     fn poll_next(
//         mut self: Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Option<Self::Item>> {
//         let a = Pin::new(&mut self.0);
//         if let Poll::Ready(x) = BroadcastStream::poll_next(a, cx) {
//             if let Some(x) = x {
//                 if let Ok(x) = x {
//                     Poll::Ready(x)
//                 } else {
//                     Poll::Ready(None)
//                 }
//             } else {
//                 Poll::Ready(None)
//             }
//         } else {
//             Poll::Pending
//         }
//     }
// }
