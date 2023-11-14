use std::pin::Pin;

use tokio_stream::Stream;
use sea_orm::sea_query::ValueType;
use sea_orm::*;
use tonic::{async_trait, Request, Response};

use super::super::tools::*;
use crate::grpc::backend::{ListRequest, Page, SortBy, TextSearchRequest};

use super::stream::*;
use super::transform::*;

pub trait IntelTrait {
    const NAME: &'static str;

    type Entity: EntityTrait + Default;

    type PartialModel: Send + 'static + PartialModelTrait;
    type PrimaryKey: ValueType + Transform<Self::Id> + Send + 'static + Into<<<Self::Entity as sea_orm::EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType>+Copy;
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
    fn ro_filter<S>(query: S, auth: Auth) -> Result<S, Error>
    where
        S: QueryFilter;
    fn sort_filter(
        query: Select<I::Entity>,
        sort_by: SortBy,
        page: Page,
        reverse: bool,
    ) -> Select<I::Entity>
    where
        SortBy: Transform<<I::Entity as EntityTrait>::Column>,
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
    fn rw_filter<S>(query: S, auth: Auth) -> Result<S, Error>
    where
        S: QueryFilter;
    fn can_create(auth: Auth) -> Result<i32, Error>;
    async fn update_model(
        model: <I::Entity as EntityTrait>::Model,
        info: I::UpdateInfo,
    ) -> Result<I::PrimaryKey, Error>;
    async fn create_model(info: I::CreateInfo, user_id: i32) -> Result<I::PrimaryKey, Error>;
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
    ) -> Result<Response<TonicStream<I::Info>>, Error>
    where
        SortBy: Transform<<I::Entity as EntityTrait>::Column>,
        I::PartialModel: Transform<I::Info>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let mut query = I::Entity::find();

        if !auth.is_root() {
            query = Self::ro_filter(query, auth)?;
        }

        let sort_by = SortBy::from_i32(request.sort_by).ok_or(Error::BadArgument("sort_by"))?;
        let page = request.page.ok_or(Error::BadArgument("page"))?;
        let reverse = request.reverse;

        let query = Self::sort_filter(query, sort_by, page, reverse);

        let stream: Pin<Box<dyn Stream<Item = Result<I::PartialModel, _>> + Send>> =
            Box::pin(query.stream_partial_model(db).await?);

        let output_stream = map_stream(stream);
        Ok(Response::new(Box::pin(output_stream)))
    }
    async fn search_by_text(
        &self,
        request: Request<TextSearchRequest>,
        text: &'static [<I::Entity as EntityTrait>::Column],
    ) -> Result<Response<TonicStream<I::Info>>, Error>
    where
        SortBy: Transform<<I::Entity as EntityTrait>::Column>,
        I::PartialModel: Transform<I::Info> + PartialModelTrait,
    {
        debug_assert!(text.len() > 0);
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let mut query = I::Entity::find();

        if !auth.is_root() {
            query = Self::ro_filter(query, auth)?;
        }

        let sort_by = SortBy::from_i32(request.sort_by).ok_or(Error::BadArgument("sort_by"))?;
        let page = request.page.ok_or(Error::BadArgument("page"))?;
        let reverse = false;

        let query = Self::sort_filter(query, sort_by, page, reverse);

        let mut condition = text[0].like(request.text.clone());
        for col in text[1..].iter() {
            condition = condition.or(col.like(request.text.clone()));
        }

        let query = query.filter(condition);

        let stream: Pin<Box<dyn Stream<Item = Result<I::PartialModel, _>> + Send>> =
            Box::pin(query.stream_partial_model(db).await?);

        let output_stream = map_stream(stream);
        Ok(Response::new(Box::pin(output_stream)))
    }
    async fn full_info<'a>(
        &'a self,
        request: Request<I::Id>,
    ) -> Result<Response<I::FullInfo>, Error>
    where
        <I::Entity as EntityTrait>::Model: AsyncTransform<Result<I::FullInfo, Error>>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let pk = Transform::into(request);
        let mut query = I::Entity::find_by_id(pk);
        if !auth.is_root() {
            query = Self::ro_filter(query, auth)?;
        }

        let model = query.one(db).await?.ok_or(Error::NotInDB(I::NAME))?;
        let info: I::FullInfo = AsyncTransform::into(model).await?;
        Ok(Response::new(info))
    }
    async fn update<R>(&self, request: tonic::Request<R>) -> Result<Response<()>, Error>
    where
        R: TryTransform<(I::UpdateInfo, I::PrimaryKey), Error> + Send,
    {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let (info, pk) = TryTransform::try_into(request)?;

        let query = match auth.is_root() {
            true => I::Entity::find_by_id(pk),
            false => Self::rw_filter(I::Entity::find_by_id(pk), auth)?,
        };

        let model = query.one(db).await?.ok_or(Error::NotInDB(I::NAME))?;

        Self::update_model(model, info).await?;

        Ok(Response::new(()))
    }
    async fn create<R>(&self, request: tonic::Request<R>) -> Result<Response<I::Id>, Error>
    where
        R: TryTransform<I::CreateInfo, Error> + Send,
    {
        let (auth, request) = self.parse_request(request).await?;
        let info = request.try_into()?;

        let user_id = match auth.is_root() {
            true => auth.user_id().unwrap(),
            false => Self::can_create(auth)?,
        };

        let pk = Self::create_model(info, user_id).await?;
        Ok(Response::new(Transform::into(pk)))
    }
    async fn remove(&self, request: tonic::Request<I::Id>) -> Result<Response<()>, Error> {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let pk = Transform::into(request);

        let query = match auth.is_root() {
            true => I::Entity::delete_by_id(pk),
            false => Self::rw_filter(I::Entity::delete_by_id(pk), auth)?,
        };

        match query.exec(db).await?.rows_affected {
            0 => Err(Error::NotInDB(I::NAME)),
            _ => Ok(Response::new(())),
        }
    }
}
