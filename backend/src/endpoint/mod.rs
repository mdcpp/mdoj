use migration::async_trait;
use thiserror::Error;
use tonic::async_trait;

use crate::{common::prelude::UserPermBytes, Server};

pub mod problem;
pub mod testcase;
pub mod util;

#[derive(Debug, Error)]
pub enum Error {
    #[error("`{0}`")]
    Upstream(#[from] crate::controller::Error),
    #[error("Premission Deny")]
    PremissionDeny,
}

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
    pub fn ok_or(&self, err: tonic::Status) -> Result<(i32, UserPermBytes), tonic::Status> {
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
    pub fn ok_or_default(&self) -> Result<(i32, UserPermBytes), tonic::Status> {
        self.ok_or(tonic::Status::unauthenticated("Permission Deny"))
    }
}

#[async_trait]
pub trait ControllerTrait {
    async fn parse_request<T>(
        &self,
        request: tonic::Request<T>,
    ) -> Result<(Auth, T), tonic::Status>
    where
        T: Send;
}

#[async_trait]
impl ControllerTrait for Server {
    async fn parse_request<T>(&self, request: tonic::Request<T>) -> Result<(Auth, T), tonic::Status>
    where
        T: Send,
    {
        let (meta, _, payload) = request.into_parts();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();

            match self.controller.verify(token).await.map_err(|x| {
                log::error!("Token verification failed: {}", x);
                tonic::Status::unauthenticated("Token verification failed")
            })? {
                Some(x) => Ok((Auth::User(x), payload)),
                None => Err(tonic::Status::unauthenticated("Token does not exist")),
            }
        } else {
            Ok((Auth::Guest, payload))
        }
    }
}
