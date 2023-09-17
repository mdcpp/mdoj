use migration::ValueType;
use sea_orm::*;
use tonic::{async_trait, Request, Response};

use crate::common::error::result_into;
use crate::endpoint::*;
use crate::grpc::proto::prelude::{ListRequest, Page, SortBy, TextSearchRequest};
use crate::init::db::DB;

use super::transform::{Transform, TryTransform};

pub trait IntelTrait {
    type Entity: EntityTrait;

    type PartialModel: PartialModelTrait;
    type PrimaryKey: ValueType;
    type Id: Transform<Self::PrimaryKey> + Send + 'static;

    type InfoArray;
    type FullInfo;
    type Info;

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
    fn can_create(auth: Auth) -> Result<(), tonic::Status>;
    async fn update_model(
        model: <<I as IntelTrait>::Entity as EntityTrait>::Model,
        info: <I as IntelTrait>::UpdateInfo,
    ) -> Result<<I as IntelTrait>::PrimaryKey, tonic::Status>;
    async fn create_model(
        model: <I as IntelTrait>::CreateInfo,
    ) -> Result<<I as IntelTrait>::PrimaryKey, tonic::Status>;
}

#[async_trait]
pub trait BaseEndpoint<I>
where
    I: IntelTrait,
    Self: Intel<I> + ControllerTrait,
    <I as IntelTrait>::PrimaryKey :Transform<<I as IntelTrait>::Id>+Send,
    <<<I as IntelTrait>::Entity as sea_orm::EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<<I as IntelTrait>::PrimaryKey>
{
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<<I as IntelTrait>::InfoArray>, tonic::Status>
    where
        SortBy: Transform<<<I as IntelTrait>::Entity as EntityTrait>::Column>,
        Vec<<I as IntelTrait>::Info>: Transform<<I as IntelTrait>::InfoArray>,
        <I as IntelTrait>::PartialModel: Transform<<I as IntelTrait>::Info> + Send,
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
            result_into(query.into_partial_model().all(db).await)?;
        let list: Vec<<I as IntelTrait>::Info> =
            list.into_iter().map(|x| Transform::into(x)).collect();
        Ok(Response::new(Transform::into(list)))
    }
    async fn search_by_text(
        &self,
        request: Request<TextSearchRequest>,
        text: &'static [<<I as IntelTrait>::Entity as EntityTrait>::Column],
    ) -> Result<Response<<I as IntelTrait>::InfoArray>, tonic::Status>
    where
        SortBy: Transform<<<I as IntelTrait>::Entity as EntityTrait>::Column>,
        Vec<<I as IntelTrait>::Info>: Transform<<I as IntelTrait>::InfoArray>,
        <I as IntelTrait>::PartialModel: Transform<<I as IntelTrait>::Info> + Send,
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
            result_into(query.into_partial_model().all(db).await)?;
        let list: Vec<<I as IntelTrait>::Info> =
            list.into_iter().map(|x| Transform::into(x)).collect();

        Ok(Response::new(Transform::into(list)))
    }
    async fn full_info(
        &self,
        request: Request<<I as IntelTrait>::Id>,
    ) -> Result<Response<<I as IntelTrait>::FullInfo>, tonic::Status>
    where
        <<I as IntelTrait>::Entity as EntityTrait>::Model: Transform<<I as IntelTrait>::FullInfo>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let pk = Transform::into(request);
        let query = I::Entity::find_by_id(pk);
        let query = Self::ro_filter(query, auth)?;

        let info: <I as IntelTrait>::FullInfo = Transform::into(
            result_into(query.one(db).await)?.ok_or(tonic::Status::not_found("Not found"))?,
        );
        Ok(Response::new(info))
    }
    async fn update<R>(
        &self,
        request: tonic::Request<R>,
    ) -> Result<Response<()>, tonic::Status>
    where
        R: TryTransform<(<I as IntelTrait>::UpdateInfo,<I as IntelTrait>::PrimaryKey),tonic::Status>+Send,
    {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let (info, pk) = TryTransform::try_into(request)?;

        let query = Self::rw_filter(I::Entity::find_by_id(pk), auth)?;

        let model=result_into(query.one(db).await)?.ok_or(tonic::Status::not_found("message"))?;

        Self::update_model(model, info).await?;

        Ok(Response::new(()))
    }
    async fn create<R>(
        &self,
        request: tonic::Request<R>,
    ) -> Result<Response<<I as IntelTrait>::Id>, tonic::Status>
    where
        R: TryTransform<<I as IntelTrait>::CreateInfo,tonic::Status>+Send,
    {
        let (auth, request) = self.parse_request(request).await?;
        let info = request.try_into()?;

        Self::can_create(auth)?;

        let pk=Self::create_model(info).await?;
        Ok(Response::new(Transform::into(pk)))
    }
    async fn remove(
        &self,
        request: tonic::Request<<I as IntelTrait>::Id>,
    ) -> Result<Response<()>, tonic::Status>
    {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let pk= Transform::into(request);

        let query = Self::rw_filter(I::Entity::delete_by_id(pk), auth)?;

        match result_into(query.exec(db).await)?.rows_affected{
            0 => Err(tonic::Status::not_found("message")),
            _ => Ok(Response::new(())),
        }
    }
}
