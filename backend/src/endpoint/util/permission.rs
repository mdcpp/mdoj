use crate::{common::prelude::UserPermBytes, Server};

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
}

impl Server {
    pub async fn parse_request<T>(
        &self,
        request: tonic::Request<T>,
    ) -> Result<(Auth, T), tonic::Status> {
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
