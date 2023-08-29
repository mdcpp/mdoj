// use futures::Future;\

use tonic::async_trait;

use crate::common::prelude::UserPermBytes;

pub struct ListRequest<S, F> {
    page: i32,
    amount: i32,
    sortby: S,
    filter: F,
}

pub trait Intellisense {
    type SortBy;
    type Filter;
    type ShortInfo;
    type FullInfo;
}

pub struct UserInfo {
    perm: UserPermBytes,
    user_id: i32,
}

#[async_trait]
pub trait Queryable<I>
where
    I: Intellisense,
    I::SortBy: Send,
    I::Filter: Send,
    I::ShortInfo: Send,
    I::FullInfo: Send,
{
    type Error;

    async fn list<R, O>(
        &self,
        request: R,
        user: Option<UserInfo>,
    ) -> Result<O, Self::Error>
    where
        R: Into<ListRequest<I::SortBy, I::Filter>> + Send,
        O: From<Vec<I::ShortInfo>>+ Send,
    {
        self.raw_list(request.into(), user).await.map(|x|x.into())
    }

    async fn full_info<R, O>(
        &self,
        request: i32,
        user: Option<UserInfo>,
    ) -> Result<Option<O>, Self::Error>
    where
        R: Into<i32> + Send,
        O: From<I::FullInfo>+Send,
    {
        self.raw_full_info(request.into(), user)
            .await
            .map(|x| x.map(|x| x.into()))
    }

    async fn raw_list(
        &self,
        request: ListRequest<I::SortBy, I::Filter>,
        user: Option<UserInfo>,
    ) -> Result<Vec<I::ShortInfo>, Self::Error>;

    async fn raw_full_info(
        &self,
        request: i32,
        user: Option<UserInfo>,
    ) -> Result<Option<I::FullInfo>, Self::Error>;
}

pub trait Editer{

}

pub trait Editable<E>where E:Editer {
    
}