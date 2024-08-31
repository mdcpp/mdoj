pub use grpc::backend::*;
use tonic::{metadata::MetadataMap, IntoRequest, Request};

pub trait WithToken: Sized {
    /// this will add token to request.
    fn with_token(self, token: impl AsRef<str>) -> Request<Self>;
}

impl<T> WithToken for T
where
    T: IntoRequest<T>,
{
    fn with_token(self, token: impl AsRef<str>) -> Request<Self> {
        let mut req = self.into_request();
        let Ok(token) = token.as_ref().parse() else {
            return req;
        };
        let mut metadata = MetadataMap::new();
        metadata.insert("token", token);
        *req.metadata_mut() = metadata;
        req
    }
}
