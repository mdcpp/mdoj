//! an abtraction for pager(paginator with state)
//!
//! When using this module, we follow steps below:
//!
//! - construct pager
//!     - Get filter constructor
//!     - Deserialize byte for PageState
//! - Fetch data and return data(`PagerSource`) to frontend
//! - Update pager state(`PagerReflect`)
//! - Serialize and return pager

use super::helper::*;
use crate::util::auth::Auth;
use sea_orm::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tonic::async_trait;
use tracing::instrument;

use std::marker::PhantomData;

use sea_orm::{ColumnTrait, EntityTrait, QuerySelect, Select};

use crate::util::error::Error;

const PAGINATE_GUARANTEE: u64 = 96;

#[async_trait]
pub trait Pager
where
    Self: Sized + Serialize + DeserializeOwned,
{
    type Source: PagerData;
    type Reflect: Send;
    async fn fetch(
        self,
        auth: &Auth,
        size: u64,
        offset: u64,
        rel_dir: bool,
        db: &DatabaseConnection,
    ) -> Result<(Self, Vec<Self::Reflect>), Error>;
    async fn new_fetch(
        data: <Self::Source as PagerData>::Data,
        auth: &Auth,
        size: u64,
        offset: u64,
        abs_dir: bool,
        db: &DatabaseConnection,
    ) -> Result<(Self, Vec<Self::Reflect>), Error>;
}

#[async_trait]
pub trait Remain {
    async fn remain(&self, auth: &Auth, db: &DatabaseConnection) -> Result<u64, Error>;
}

pub trait PagerData {
    type Data: Send + Sized + Serialize + DeserializeOwned + Sync;
}
/// indicate foreign object is ready for page source
///
/// In `Education` example, we expect ::entity::education::Entity
/// to implement it
#[async_trait]
pub trait Source
where
    Self: Send + PagerData,
{
    const ID: <Self::Entity as EntityTrait>::Column;
    type Entity: EntityTrait;
    const TYPE_NUMBER: u8;
    /// filter reconstruction
    async fn filter(
        auth: &Auth,
        data: &Self::Data,
        db: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error>;
}

/// indicate foreign object is ready for page reflect
///
/// In `Education` example, we expect ::entity::education::PartialEducation
/// to implement it
#[async_trait]
pub trait Reflect<E: EntityTrait>
where
    Self: Sized + Send,
{
    /// get id of primary key
    fn get_id(&self) -> i32;
    async fn all(query: Select<E>, db: &DatabaseConnection) -> Result<Vec<Self>, Error>;
}

/// compact primary key pager
#[derive(Serialize, Deserialize)]
pub struct PrimaryKeyPaginator<S: Source, R: Reflect<S::Entity>> {
    /// last primary
    last_id: i32,
    /// last direction(relative)
    last_direction: bool,
    /// original direction
    direction: bool,
    /// data
    #[serde(bound(deserialize = "S::Data: DeserializeOwned"))]
    #[serde(bound(serialize = "S::Data: Serialize"))]
    data: S::Data,
    #[serde(bound(serialize = ""))]
    #[serde(bound(deserialize = ""))]
    source: PhantomData<S>,
    #[serde(bound(serialize = ""))]
    #[serde(bound(deserialize = ""))]
    reflect: PhantomData<R>,
}
#[async_trait]
impl<S: Source + Sync, R: Reflect<S::Entity> + Sync> Remain for PrimaryKeyPaginator<S, R> {
    async fn remain(&self, auth: &Auth, db: &DatabaseConnection) -> Result<u64, Error> {
        let paginator = PaginatePkBuilder::default()
            .pk(<S as Source>::ID)
            .include(false)
            .rev(self.direction)
            .last_pk(self.last_id)
            .build()
            .unwrap();

        let query = paginator.apply(S::filter(auth, &self.data, db).await?);

        let max_count = MaxCountBuilder::default()
            .query(query)
            .max(PAGINATE_GUARANTEE)
            .build()
            .unwrap();
        max_count.count(db).await
    }
}

#[async_trait]
impl<S: Source, R: Reflect<S::Entity>> Pager for PrimaryKeyPaginator<S, R> {
    type Source = S;
    type Reflect = R;

    async fn fetch(
        mut self,
        auth: &Auth,
        size: u64,
        offset: u64,
        rel_dir: bool,
        db: &DatabaseConnection,
    ) -> Result<(Self, Vec<R>), Error> {
        // FIXME: should use PassThrough
        let paginator = PaginatePkBuilder::default()
            .pk(<S as Source>::ID)
            .include(self.last_direction ^ rel_dir)
            .rev(self.direction ^ rel_dir)
            .last_pk(self.last_id)
            .build()
            .unwrap();

        self.last_direction = rel_dir;

        let query = paginator
            .apply(S::filter(auth, &self.data, db).await?)
            .limit(size)
            .offset(offset);
        let models = R::all(query, db).await?;

        // FIXME: should check not found
        if let Some(model) = models.last() {
            self.last_id = R::get_id(model);
            return Ok((self, models));
        }

        Err(Error::NotInDBList)
    }
    async fn new_fetch(
        data: S::Data,
        auth: &Auth,
        _size: u64,
        _offset: u64,
        abs_dir: bool,
        db: &DatabaseConnection,
    ) -> Result<(Self, Vec<R>), Error> {
        let query = order_by_bool(
            S::filter(auth, &data, db).await?,
            <S as Source>::ID,
            abs_dir,
        );

        let models = R::all(query, db).await?;

        // FIXME: should check not found
        if let Some(model) = models.last() {
            return Ok((
                Self {
                    last_id: R::get_id(model),
                    last_direction: abs_dir,
                    direction: abs_dir,
                    data,
                    source: PhantomData,
                    reflect: PhantomData,
                },
                models,
            ));
        }

        Err(Error::NotInDBList)
    }
}

pub trait SortSource<R>
where
    Self: Source,
{
    /// get sort column
    fn sort_col(data: &Self::Data) -> impl ColumnTrait;
    /// get value of column
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send;
    /// save last value in column
    fn save_val(data: &mut Self::Data, model: &R);
}

/// compact column pager
#[derive(Serialize, Deserialize)]
pub struct ColumnPaginator<S: SortSource<R>, R: Reflect<S::Entity>> {
    /// last primary
    last_id: i32,
    /// last direction(relative)
    last_direction: bool,
    /// original direction
    direction: bool,
    #[serde(bound(deserialize = "S::Data: DeserializeOwned"))]
    #[serde(bound(serialize = "S::Data: Serialize"))]
    data: S::Data,
    #[serde(bound(deserialize = ""))]
    #[serde(bound(serialize = ""))]
    source: PhantomData<S>,
    #[serde(bound(deserialize = ""))]
    #[serde(bound(serialize = ""))]
    reflect: PhantomData<R>,
}

#[async_trait]
impl<S: SortSource<R> + Sync, R: Reflect<S::Entity> + Sync> Remain for ColumnPaginator<S, R> {
    async fn remain(&self, auth: &Auth, db: &DatabaseConnection) -> Result<u64, Error> {
        let col = S::sort_col(&self.data);
        let val = S::get_val(&self.data);

        let paginator = PaginateColBuilder::default()
            .pk(<S as Source>::ID)
            .include(false)
            .rev(self.direction)
            .col(col)
            .last_value(val)
            .last_pk(self.last_id)
            .build()
            .unwrap();

        let query = paginator.apply(S::filter(auth, &self.data, db).await?);

        let max_count = MaxCountBuilder::default()
            .query(query)
            .max(PAGINATE_GUARANTEE)
            .build()
            .unwrap();
        max_count.count(db).await
    }
}

#[async_trait]
impl<S: SortSource<R>, R: Reflect<S::Entity>> Pager for ColumnPaginator<S, R> {
    type Source = S;
    type Reflect = R;

    #[instrument(skip(self), level = "debug")]
    async fn fetch(
        mut self,
        auth: &Auth,
        size: u64,
        offset: u64,
        rel_dir: bool,
        db: &DatabaseConnection,
    ) -> Result<(Self, Vec<R>), Error> {
        let col = S::sort_col(&self.data);
        let val = S::get_val(&self.data);

        let paginator = PaginateColBuilder::default()
            .pk(<S as Source>::ID)
            .include(self.last_direction ^ rel_dir)
            .rev(self.direction ^ rel_dir)
            .col(col)
            .last_value(val)
            .last_pk(self.last_id)
            .build()
            .unwrap();

        self.last_direction = rel_dir;

        let query = paginator
            .apply(S::filter(auth, &self.data, db).await?)
            .limit(size)
            .offset(offset);
        let models = R::all(query, db).await?;

        if size as usize != models.len() {
            tracing::debug!(size = size, len = models.len(), "miss_data")
        }

        if let Some(model) = models.last() {
            S::save_val(&mut self.data, model);
            self.last_id = R::get_id(model);
            return Ok((self, models));
        }

        Err(Error::NotInDBList)
    }
    #[instrument(skip(data), level = "debug")]
    async fn new_fetch(
        mut data: S::Data,
        auth: &Auth,
        size: u64,
        offset: u64,
        abs_dir: bool,
        db: &DatabaseConnection,
    ) -> Result<(Self, Vec<R>), Error> {
        let col = S::sort_col(&data);

        let query = order_by_bool(
            S::filter(auth, &data, db).await?,
            <S as Source>::ID,
            abs_dir,
        );
        let query = order_by_bool(query, col, abs_dir)
            .limit(size)
            .offset(offset);

        let models = R::all(query, db).await?;

        if size as usize != models.len() {
            tracing::debug!(size = size, len = models.len(), "miss_data")
        }
        // FIXME: should check not found
        if let Some(model) = models.last() {
            S::save_val(&mut data, model);

            return Ok((
                Self {
                    last_id: R::get_id(model),
                    last_direction: abs_dir,
                    direction: abs_dir,
                    data,
                    source: PhantomData,
                    reflect: PhantomData,
                },
                models,
            ));
        }

        Err(Error::NotInDBList)
    }
}

pub(super) trait Paginate<E: EntityTrait> {
    /// Apply pagination effect on a Select(sea_orm)
    ///
    /// be careful not to run order_by before applying pagination
    fn apply(self, query: Select<E>) -> Select<E>;
}
