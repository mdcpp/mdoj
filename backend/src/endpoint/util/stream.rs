use std::{error::Error, pin::Pin};

use tokio::sync::mpsc::*;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

pub fn into_tokiostream<O: Send + 'static>(
    mut iter: impl Iterator<Item = O> + Send + 'static,
) -> TonicStream<O> {
    let (tx, rx) = channel(128);

    tokio::spawn(async move {
        for item in iter.by_ref() {
            if tx.send(Result::<_, tonic::Status>::Ok(item)).await.is_err() {
                break;
            }
        }
    });

    let output_stream = ReceiverStream::new(rx);
    Box::pin(output_stream) as TonicStream<O>
}

pub fn map_stream<O, I, E>(
    mut stream: impl tokio_stream::Stream<Item = Result<I, E>> + Unpin + Send + 'static,
) -> TonicStream<O>
where
    O: Send + 'static,
    I: Send + 'static + Into<O>,
    E: Send + Error,
{
    let (tx, rx) = channel(128);

    tokio::spawn(async move {
        while let Some(item) = stream.next().await {
            if item.is_err() {
                break;
            }
            if tx
                .send(Result::<_, tonic::Status>::Ok(item.unwrap().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    Box::pin(ReceiverStream::new(rx)) as TonicStream<O>
}

pub type TonicStream<T> =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<T, tonic::Status>> + Send>>;
