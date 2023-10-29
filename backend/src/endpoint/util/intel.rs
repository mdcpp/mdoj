use std::pin::Pin;

use migration::ValueType;
use sea_orm::*;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{async_trait, Request, Response};

use crate::common::error::handle_dberr;
use crate::endpoint::*;
use crate::grpc::proto::prelude::{ListRequest, Page, SortBy, TextSearchRequest};
use crate::init::db::DB;

use super::stream::{into_tokiostream, TonicStream};
use super::transform::{AsyncTransform, Transform, TryTransform};

pub trait IntelTrait {
    type Entity: EntityTrait;

    type PartialModel: Send + 'static;
    type PrimaryKey: ValueType + Transform<<Self as IntelTrait>::Id> + Send + 'static + Into<<<<Self as IntelTrait>::Entity as sea_orm::EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType>;
    type Id: Transform<Self::PrimaryKey> + Send + 'static;

    type InfoArray;
    type FullInfo;
    type Info: Send + 'static;

    type UpdateInfo: Send;
    type CreateInfo: Send;
}

#[async_trait]
pub trait Intel<I>
where
    I: IntelTrait,
{
    fn ro_filter<S>(query: S, auth: Auth) -> Result<S, tonic::Status>
    where
        S: QueryFilter;
    fn sort_filter(
        query: Select<I::Entity>,
        sort_by: SortBy,
        page: Page,
        reverse: bool,
    ) -> Select<I::Entity>
    where
        SortBy: Transform<<<I as IntelTrait>::Entity as EntityTrait>::Column>,
    {
        let col = Transform::into(sort_by);

        let query = if reverse {
            query.order_by_asc(col)
        } else {
            query.order_by_desc(col)
        };

        let offset = page.offset as u64;
        let limit = page.amount as u64;
        query.clone().offset(offset).limit(limit)
    }
    fn rw_filter<S>(query: S, auth: Auth) -> Result<S, tonic::Status>
    where
        S: QueryFilter;
    fn can_create(auth: Auth) -> Result<i32, tonic::Status>;
    async fn update_model(
        model: <<I as IntelTrait>::Entity as EntityTrait>::Model,
        info: <I as IntelTrait>::UpdateInfo,
    ) -> Result<<I as IntelTrait>::PrimaryKey, tonic::Status>;
    async fn create_model(
        model: <I as IntelTrait>::CreateInfo,
        user_id: i32,
    ) -> Result<<I as IntelTrait>::PrimaryKey, tonic::Status>;
}

#[async_trait]
pub trait BaseEndpoint<I>
where
    I: IntelTrait,
    Self: Intel<I> + ControllerTrait,
{
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<TonicStream<<I as IntelTrait>::Info>>, tonic::Status>
    where
        SortBy: Transform<<<I as IntelTrait>::Entity as EntityTrait>::Column>,
        <I as IntelTrait>::PartialModel: Transform<<I as IntelTrait>::Info> + PartialModelTrait,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let query = I::Entity::find();
        let query = Self::ro_filter(query, auth)?;

        let sort_by = SortBy::from_i32(request.sort_by)
            .ok_or(tonic::Status::invalid_argument("Invalid sort_by"))?;
        let page = request
            .page
            .ok_or(tonic::Status::invalid_argument("Invalid page"))?;
        let reverse = request.reverse;

        let query = Self::sort_filter(query, sort_by, page, reverse);

        let list: Vec<<I as IntelTrait>::PartialModel> =
            handle_dberr(query.into_partial_model().all(db).await)?;

        let output_stream = into_tokiostream(list.into_iter().map(|x| Transform::into(x)));
        Ok(Response::new(Box::pin(output_stream)))
    }
    async fn search_by_text(
        &self,
        request: Request<TextSearchRequest>,
        text: &'static [<<I as IntelTrait>::Entity as EntityTrait>::Column],
    ) -> Result<Response<TonicStream<<I as IntelTrait>::Info>>, tonic::Status>
    where
        SortBy: Transform<<<I as IntelTrait>::Entity as EntityTrait>::Column>,
        <I as IntelTrait>::PartialModel: Transform<<I as IntelTrait>::Info> + PartialModelTrait,
    {
        debug_assert!(text.len() > 0);
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let query = I::Entity::find();
        let query = Self::ro_filter(query, auth)?;

        let sort_by = SortBy::from_i32(request.sort_by)
            .ok_or(tonic::Status::invalid_argument("Invalid sort_by"))?;
        let page = request
            .page
            .ok_or(tonic::Status::invalid_argument("Invalid page"))?;
        let reverse = false;

        let query = Self::sort_filter(query, sort_by, page, reverse);

        let mut condition = text[0].like(request.text.clone());
        for col in text[1..].iter() {
            condition = condition.or(col.like(request.text.clone()));
        }

        let query = query.filter(condition);

        let list: Vec<<I as IntelTrait>::PartialModel> =
            handle_dberr(query.into_partial_model().all(db).await)?;

        let output_stream = into_tokiostream(list.into_iter().map(|x| Transform::into(x)));
        Ok(Response::new(Box::pin(output_stream)))
    }
    async fn full_info(
        &self,
        request: Request<<I as IntelTrait>::Id>,
    ) -> Result<Response<<I as IntelTrait>::FullInfo>, tonic::Status>
    where
        <<I as IntelTrait>::Entity as EntityTrait>::Model:
            AsyncTransform<Result<<I as IntelTrait>::FullInfo, tonic::Status>>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let pk = Transform::into(request);
        let query = I::Entity::find_by_id(pk);
        let query = Self::ro_filter(query, auth)?;
        let model =
            handle_dberr(query.one(db).await)?.ok_or(tonic::Status::not_found("Not found"))?;
        let info: <I as IntelTrait>::FullInfo = AsyncTransform::into(model).await?;
        Ok(Response::new(info))
    }
    async fn update<R>(&self, request: tonic::Request<R>) -> Result<Response<()>, tonic::Status>
    where
        R: TryTransform<
                (<I as IntelTrait>::UpdateInfo, <I as IntelTrait>::PrimaryKey),
                tonic::Status,
            > + Send,
    {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let (info, pk) = TryTransform::try_into(request)?;

        let query = Self::rw_filter(I::Entity::find_by_id(pk), auth)?;

        let model =
            handle_dberr(query.one(db).await)?.ok_or(tonic::Status::not_found("message"))?;

        Self::update_model(model, info).await?;

        Ok(Response::new(()))
    }
    async fn create<R>(
        &self,
        request: tonic::Request<R>,
    ) -> Result<Response<<I as IntelTrait>::Id>, tonic::Status>
    where
        R: TryTransform<<I as IntelTrait>::CreateInfo, tonic::Status> + Send,
    {
        let (auth, request) = self.parse_request(request).await?;
        let info = request.try_into()?;

        let user_id = Self::can_create(auth)?;

        let pk = Self::create_model(info, user_id).await?;
        Ok(Response::new(Transform::into(pk)))
    }
    async fn remove(
        &self,
        request: tonic::Request<<I as IntelTrait>::Id>,
    ) -> Result<Response<()>, tonic::Status> {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let pk = Transform::into(request);

        let query = Self::rw_filter(I::Entity::delete_by_id(pk), auth)?;

        match handle_dberr(query.exec(db).await)?.rows_affected {
            0 => Err(tonic::Status::not_found("")),
            _ => Ok(Response::new(())),
        }
    }
}
