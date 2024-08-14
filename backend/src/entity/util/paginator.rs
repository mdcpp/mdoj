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
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use tonic::async_trait;
use tracing::*;

use std::marker::PhantomData;

use crate::util::error::Error;

const PAGINATE_GUARANTEE: u64 = 96;

#[async_trait]
pub trait PaginateRaw
where
    Self: Sized + Serialize + DeserializeOwned,
{
    type Source: PagerData;
    type Reflect: Send;
    async fn fetch(
        &mut self,
        auth: &Auth,
        size: i64,
        offset: u64,
        db: &DatabaseConnection,
    ) -> Result<Vec<Self::Reflect>, Error>;
    async fn new_fetch(
        data: <Self::Source as PagerData>::Data,
        auth: &Auth,
        size: u64,
        offset: u64,
        start_from_end: bool,
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
impl<S: Source, R: Reflect<S::Entity>> PaginateRaw for PrimaryKeyPaginator<S, R> {
    type Source = S;
    type Reflect = R;

    #[instrument(skip(self, db), level = "debug", name = "query_paginator")]
    async fn fetch(
        &mut self,
        auth: &Auth,
        size: i64,
        offset: u64,
        db: &DatabaseConnection,
    ) -> Result<Vec<R>, Error> {
        let dir = size.is_negative();
        // FIXME: should use PassThrough
        let paginator = PaginatePkBuilder::default()
            .pk(<S as Source>::ID)
            .include(self.last_direction ^ dir)
            .rev(self.direction ^ dir)
            .last_pk(self.last_id)
            .build()
            .unwrap();

        self.last_direction = dir;

        let query = paginator
            .apply(S::filter(auth, &self.data, db).await?)
            .limit(size.unsigned_abs())
            .offset(offset);
        let models = R::all(query, db).await?;

        if let Some(model) = models.last() {
            self.last_id = R::get_id(model);
            return Ok(models);
        }

        Err(Error::NotInDBList)
    }
    #[instrument(skip(data, db), level = "debug", rename = "create_paginator")]
    async fn new_fetch(
        data: S::Data,
        auth: &Auth,
        size: u64,
        offset: u64,
        abs_dir: bool,
        db: &DatabaseConnection,
    ) -> Result<(Self, Vec<R>), Error> {
        let query = order_by_bool(
            S::filter(auth, &data, db).await?,
            <S as Source>::ID,
            abs_dir,
        )
        .limit(size)
        .offset(offset);

        let models = R::all(query, db).await?;

        if let Some(model) = models.last() {
            return Ok((
                Self {
                    last_id: R::get_id(model),
                    last_direction: false,
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
    #[instrument(skip_all, level = "debug", name = "count_remain")]
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
impl<S: SortSource<R>, R: Reflect<S::Entity>> PaginateRaw for ColumnPaginator<S, R> {
    type Source = S;
    type Reflect = R;

    #[instrument(skip(self, db), level = "debug", name = "query_paginator")]
    async fn fetch(
        &mut self,
        auth: &Auth,
        size: i64,
        offset: u64,
        db: &DatabaseConnection,
    ) -> Result<Vec<R>, Error> {
        let dir = size.is_negative();

        let col = S::sort_col(&self.data);
        let val = S::get_val(&self.data);

        let paginator = PaginateColBuilder::default()
            .pk(<S as Source>::ID)
            .include(self.last_direction ^ dir)
            .rev(self.direction ^ dir)
            .col(col)
            .last_value(val)
            .last_pk(self.last_id)
            .build()
            .unwrap();

        self.last_direction = dir;

        let query = paginator
            .apply(S::filter(auth, &self.data, db).await?)
            .limit(size.unsigned_abs())
            .offset(offset);
        let models = R::all(query, db)
            .instrument(debug_span!("query").or_current())
            .await?;

        if size as usize != models.len() {
            tracing::debug!(size = size, len = models.len(), "miss_data")
        }

        let _enter = debug_span!("save_state").or_current().entered();

        if let Some(model) = models.last() {
            S::save_val(&mut self.data, model);
            self.last_id = R::get_id(model);
            return Ok(models);
        }

        Err(Error::NotInDBList)
    }
    #[instrument(skip(data, db), level = "debug", rename = "create_paginator")]
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
            S::filter(auth, &data, db).in_current_span().await?,
            <S as Source>::ID,
            abs_dir,
        );
        let query = order_by_bool(query, col, abs_dir)
            .limit(size)
            .offset(offset);

        let models = R::all(query, db).in_current_span().await?;

        if size as usize != models.len() {
            tracing::trace!(size = size, len = models.len(), "miss_data")
        }
        if let Some(model) = models.last() {
            S::save_val(&mut data, model);

            return Ok((
                Self {
                    last_id: R::get_id(model),
                    last_direction: false,
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

#[derive(Serialize, Default)]
pub enum UninitPaginator<P: PaginateRaw> {
    #[serde(skip)]
    Uninit(<P::Source as PagerData>::Data, bool),
    #[serde(bound(deserialize = "P: for<'a> Deserialize<'a>"))]
    Init(P),
    #[serde(skip)]
    #[default]
    None,
}

impl<'de, P: PaginateRaw> Deserialize<'de> for UninitPaginator<P> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p: Option<P> = Deserialize::deserialize(deserializer)?;

        match p {
            Some(p) => Ok(UninitPaginator::Init(p)),
            None => Err(serde::de::Error::custom(
                "Unexpected data format for UninitPaginator",
            )),
        }
    }
}

impl<P: PaginateRaw> UninitPaginator<P> {
    pub fn new(data: <P::Source as PagerData>::Data, start_from_end: bool) -> Self {
        Self::Uninit(data, start_from_end)
    }
    pub async fn fetch(
        &mut self,
        size: u64,
        offset: i64,
        auth: &Auth,
        db: &DatabaseConnection,
    ) -> Result<Vec<P::Reflect>, Error> {
        if let UninitPaginator::Init(x) = self {
            let size = size.min((i64::MAX - 1) as u64) as i64;
            let offset = offset.max(i64::MIN + 1);
            let (size, offset) = match offset < 0 {
                true => (
                    -size,
                    (-offset)
                        .checked_sub(size)
                        .ok_or(Error::BadArgument("size"))? as u64,
                ),
                false => (size, offset as u64),
            };
            x.fetch(auth, size, offset, db).await.map(|mut x| {
                if size.is_negative() {
                    x.reverse();
                }
                x
            })
        } else if let UninitPaginator::Uninit(x, start_from_end) = std::mem::take(self) {
            let (paginator, list) =
                P::new_fetch(x, auth, size, offset.max(0) as u64, start_from_end, db).await?;
            *self = UninitPaginator::Init(paginator);
            Ok(list)
        } else {
            unreachable!("Cannot in middle state")
        }
    }
    pub async fn remain(&self, auth: &Auth, db: &DatabaseConnection) -> Result<u64, Error>
    where
        P: Remain,
    {
        match self {
            UninitPaginator::Uninit(_, _) => todo!("Handle uninited tracing"),
            UninitPaginator::Init(x) => x.remain(auth, db).await,
            UninitPaginator::None => unreachable!("Cannot in middle state"),
        }
    }
}
