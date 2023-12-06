use super::{ConnectionDetail, Error, Routable, RouteStatus};
use crate::init::config::Judger;
use tonic::transport::Uri;

pub struct StaticRouter {
    uri: Option<String>,
    secret: Option<String>,
}

#[tonic::async_trait]
impl Routable for StaticRouter {
    async fn route(&mut self) -> Result<RouteStatus, Error> {
        Ok(match self.uri.take() {
            Some(x) => RouteStatus::NewConnection(ConnectionDetail {
                uri: x,
                secret: self.secret.clone(),
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
