// use futures::Future;\

use tonic::{async_trait, codegen::http::request};

use crate::common::prelude::UserPermBytes;

pub struct ListRequest<S, F> {
    page: i32,
    amount: i32,
    sortby: S,
    filter: F,
}

pub trait Intellisense
where
    Self::SortBy: Send,
    Self::Filter: Send,
    Self::Full: Send,
    Self::Short: Send,
{
    type SortBy;
    type Filter;
    type Short;
    type Full;
}

pub struct UserInfo {
    pub perm: UserPermBytes,
    pub user_id: i32,
}

// #[async_trait]
// pub trait Queryable<I>
// where
//     I: Intellisense,
// {
//     type Error;

//     // async fn list<R, O>(&self, request: R, user: Option<UserInfo>) -> Result<O, Self::Error>
//     // where
//     //     R: Into<ListRequest<I::SortBy, I::Filter>> + Send,
//     //     O: From<Vec<I::Short>> + Send,
//     // {
//     //     self.raw_list(request.into(), user).await.map(|x| x.into())
//     // }

//     // async fn full_info<R, O>(
//     //     &self,
//     //     request: i32,
//     //     user: Option<UserInfo>,
//     // ) -> Result<Option<O>, Self::Error>
//     // where
//     //     R: Into<i32> + Send,
//     //     O: From<I::Full> + Send,
//     // {
//     //     self.raw_full_info(request.into(), user)
//     //         .await
//     //         .map(|x| x.map(|x| x.into()))
//     // }

//     async fn list(
//         &self,
//         request: ListRequest<I::SortBy, I::Filter>,
//         user: Option<UserInfo>,
//     ) -> Result<Vec<I::Short>, Self::Error>;

//     async fn full_info(
//         &self,
//         request: i32,
//         user: Option<UserInfo>,
//     ) -> Result<Option<I::Full>, Self::Error>;
// }

// pub trait Editer
// where
//     Self::Require: Send,
//     Self::Update: Send,
// {
//     type Require;
//     type Update;
// }

// #[async_trait]
// pub trait Editable<E>
// where
//     E: Editer,
// {
//     type Error;

//     async fn create(&self, request: E::Require, user: UserInfo) -> Result<i32, Self::Error>;
//     async fn update(&self, request: E::Update, user: UserInfo) -> Result<i32, Self::Error>;
//     async fn remove(&self, request: i32, user: UserInfo) -> Result<i32, Self::Error>;
// }
