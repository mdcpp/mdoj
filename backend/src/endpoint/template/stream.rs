use std::pin::Pin;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub fn into_tokiostream<O: Send + 'static>(
    mut iter: impl Iterator<Item = O> + Send + 'static,
) -> TonicStream<O> {
    let (tx, rx) = mpsc::channel(128);

    tokio::spawn(async move {
        while let Some(item) = iter.next() {
            if tx.send(Result::<_, tonic::Status>::Ok(item)).await.is_err() {
                break;
            }
        }
    });

    let output_stream = ReceiverStream::new(rx);
    Box::pin(output_stream) as TonicStream<O>
}

pub type TonicStream<T> =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<T, tonic::Status>> + Send>>;
