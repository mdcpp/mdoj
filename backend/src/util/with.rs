use crate::util::auth::Auth;
use sea_orm::DatabaseConnection;
use tonic::Response;

#[derive(Debug)]
pub struct WithAuth<'a, T>(pub &'a Auth, pub T);
#[derive(Debug)]
pub struct WithDB<'a, T>(pub &'a DatabaseConnection, pub T);

pub trait WithAuthTrait
where
    Self: Sized,
{
    fn with_auth(self, auth: &Auth) -> WithAuth<Self> {
        WithAuth(auth, self)
    }
}

pub trait WithDBTrait
where
    Self: Sized,
{
    fn with_db(self, db: &DatabaseConnection) -> WithDB<Self> {
        WithDB(db, self)
    }
}

impl<'a, 'b, T> WithDB<'a, WithAuth<'b, T>> {
    pub fn into_inner(self) -> T {
        self.1 .1
    }
}

impl<T: Sized> WithDBTrait for WithAuth<'_, T> {}

/// A newtype wrapper for crate::Result to tonic::Result;
pub struct WithGrpc<T>(crate::util::error::Result<T>);

impl<T> From<WithGrpc<T>> for tonic::Result<Response<T>> {
    fn from(value: WithGrpc<T>) -> Self {
        match value.0 {
            Ok(x) => Ok(Response::new(x)),
            Err(err) => Err(err.into()),
        }
    }
}

pub trait WithGrpcTrait {
    type Item;
    fn with_grpc(self) -> WithGrpc<Self::Item>;
}

impl<T> WithGrpcTrait for crate::util::error::Result<T> {
    type Item = T;
    fn with_grpc(self) -> WithGrpc<Self::Item> {
        WithGrpc(self)
    }
}
