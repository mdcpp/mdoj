use sea_orm::{QueryOrder, QuerySelect};
use sea_orm::{EntityTrait, Select};
use tonic::{async_trait, Request, Response};

use crate::common::error::result_into;
use crate::endpoint::{Auth, ControllerTrait};
use crate::grpc::proto::prelude::{ListRequest, Page, SortBy, SearchByTextRequest};
use crate::init::db::DB;

#[async_trait]
pub trait BaseEndpoint<I>
where
    I: IntellisenseTrait,
    Self: Intellisense<I> + ControllerTrait,
    SortBy: Into<<<I as IntellisenseTrait>::Entity as EntityTrait>::Column>,
    <I as IntellisenseTrait>::InfoArray: From<Vec<<I as IntellisenseTrait>::Info>>,
    <I as IntellisenseTrait>::Info: From<<<I as IntellisenseTrait>::Entity as EntityTrait>::Model>,
{
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<<I as IntellisenseTrait>::InfoArray>, tonic::Status> {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;
        let mut query = self.get_filter(auth)?;

        let sort_by = SortBy::from_i32(request.sort_by)
            .ok_or(tonic::Status::invalid_argument("Invalid sort_by"))?;
        let page = request
            .page
            .ok_or(tonic::Status::invalid_argument("Invalid page"))?;
        let reverse = request.reverse;

        Self::apply(&mut query, sort_by, page, reverse);

        let db_r = query.all(db).await;

        let list: Vec<<I as IntellisenseTrait>::Info> =
            result_into(db_r)?.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(list.into()))
    }
    async fn search_by_text(
        &self,
        request: Request<SearchByTextRequest>,
    ) -> Result<Response<<I as IntellisenseTrait>::InfoArray>, tonic::Status> {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;
        let mut query = self.get_filter(auth)?;

        let sort_by = SortBy::from_i32(request.sort_by)
        .ok_or(tonic::Status::invalid_argument("Invalid sort_by"))?;
        let page = request
            .page
            .ok_or(tonic::Status::invalid_argument("Invalid page"))?;
        let reverse = false;

        Self::apply(&mut query, sort_by, page, reverse);


        todo!()
    }
}

pub trait IntellisenseTrait
where
    Self: EntityTrait,
{
    type Entity: EntityTrait;

    type InfoArray;
    type FullInfo;
    type Info;
}

pub trait Intellisense<T>
where
    T: IntellisenseTrait,
{
    fn get_filter(&self, auth: Auth) -> Result<Select<T::Entity>, tonic::Status>;
    fn apply(self_: &mut Select<T::Entity>, sort_by: SortBy, page: Page, reverse: bool)
    where
        SortBy: Into<<<T as IntellisenseTrait>::Entity as EntityTrait>::Column>,
    {
        let col = sort_by.into();

        *self_ = if reverse {
            self_.clone().order_by_asc(col)
        } else {
            self_.clone().order_by_desc(col)
        };

        let offset = page.offset as u64;
        let limit = page.amount as u64;
        *self_ = self_.clone().offset(offset).limit(limit);
    }
}
