use tonic::async_trait;

use super::{error::Error, ControllerTrait};
use crate::{controller::token::UserPermBytes, init::config::CONFIG, Server};
use tracing::{span, Level};

pub enum Auth {
    Guest,
    User((i32, UserPermBytes)),
}

impl Auth {
    pub fn is_guest(&self) -> bool {
        match self {
            Auth::Guest => true,
            _ => false,
        }
    }
    pub fn user_perm(&self) -> Option<UserPermBytes> {
        match self {
            Auth::User((_, x)) => Some(*x),
            _ => None,
        }
    }
    pub fn user_id(&self) -> Option<i32> {
        match self {
            Auth::User((x, _)) => Some(*x),
            _ => None,
        }
    }
    pub fn ok_or(&self, err: Error) -> Result<(i32, UserPermBytes), Error> {
        match self {
            Auth::User(x) => Ok(*x),
            _ => Err(err),
        }
    }
    pub fn is_root(&self) -> bool {
        match self {
            Auth::User((uid, perm)) => perm.can_root(),
            _ => false,
        }
    }
    pub fn ok_or_default(&self) -> Result<(i32, UserPermBytes), Error> {
        self.ok_or(Error::PremissionDeny("Guest is not allow in this endpoint"))
    }
}

#[async_trait]
impl ControllerTrait for Server {
    async fn parse_request<T: Send>(&self, request: tonic::Request<T>) -> Result<(Auth, T), Error> {
        let span = span!(Level::INFO,"token_verify",addr=?request.remote_addr());
        let span = span.enter();

        let (meta, _, payload) = request.into_parts();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();

            match self.controller.verify(token).await.map_err(|x| {
                log::error!("Token verification failed: {}", x);
                Error::Unauthenticated
            })? {
                Some(x) => Ok((Auth::User(x), payload)),
                None => Err(Error::Unauthenticated),
            }
        } else {
            Ok((Auth::Guest, payload))
        }
    }
}

// X-Forwarded-For