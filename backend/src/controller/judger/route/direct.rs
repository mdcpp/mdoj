use super::{ConnectionDetail, Error, Routable, RouteStatus};
use crate::init::config::Judger;
use tonic::transport::Uri;

/// Upstream source for static(only emit once)
pub struct StaticRouter<const REUSE: bool> {
    uri: Option<String>,
    secret: Option<String>,
}

#[tonic::async_trait]
impl<const REUSE: bool> Routable for StaticRouter<REUSE> {
    async fn route(&mut self) -> Result<RouteStatus, Error> {
        Ok(match self.uri.take() {
            Some(x) => RouteStatus::NewConnection(ConnectionDetail {
                uri: x,
                secret: self.secret.clone(),
                reuse: REUSE,
            }),
            None => RouteStatus::Never,
        })
    }

    fn new(config: Judger) -> Result<Self, Error> {
        Uri::from_maybe_shared(config.name.clone()).map_err(|_| Error::UriParse)?;
        Ok(Self {
            uri: Some(config.name),
            secret: config.secret,
        })
    }
}
