pub mod impls;
mod paginate;

use std::{fmt::Debug, marker::PhantomData};

use sea_orm::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::instrument;

use crate::{
    entity::{Filter, ParentalTrait},
    init::db::DB,
    server::Server,
};
use paginate::{order_by_bool, PaginateColBuilder, PaginatePkBuilder};

use super::{auth::Auth, error::Error};

// TODO: add limit
const PAGE_MAX_SIZE: u64 = 64;
const PAGE_MAX_OFFSET: u64 = 256;

pub trait PagerMarker {}

pub struct NoParent;
pub struct HasParent<P> {
    _parent: PhantomData<P>,
}

impl PagerMarker for NoParent {}

impl<P: EntityTrait> PagerMarker for HasParent<P> {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EmptySortBy;
/// An abstract base class for Paginatable Entity
///
/// The trait enable sort, text search, search by parent Entity
pub trait PagerTrait
where
    Self: EntityTrait,
{
    const TYPE_NUMBER: i32;
    const COL_ID: Self::Column;
    const COL_TEXT: &'static [Self::Column];

    type ParentMarker: PagerMarker;
    type SortBy: Sized + Serialize + Clone + Debug + DeserializeOwned + Send + Sync + 'static;

    fn sort_value(model: &Self::Model, _sort: &Self::SortBy) -> String {
        Self::get_id(model).to_string()
    }
    fn sort_column(_sort: &Self::SortBy) -> Self::Column {
        Self::COL_ID
    }
    fn get_id(model: &Self::Model) -> i32;
    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error>;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LastValue(bool, String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SearchDep<E: PagerTrait> {
    Text(String),
    Column(E::SortBy, LastValue),
    Parent(i32),
    ParentSort(i32, E::SortBy, LastValue),
}

impl<E: PagerTrait> SearchDep<E> {
    fn update_last_col(&mut self, data: LastValue) {
        if let Self::Column(_, val) = self {
            *val = data;
        } else {
            unreachable!()
        }
    }
}

/// An instance of paginator itself
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pager<E: PagerTrait> {
    type_number: i32,
    #[serde(bound(deserialize = "SearchDep<E>: DeserializeOwned"))]
    #[serde(bound(serialize = "SearchDep<E>: Serialize"))]
    sort: SearchDep<E>,
    last_pk: Option<i32>,
    last_rev: bool,
}

#[tonic::async_trait]
pub trait HasParentPager<P, E>
where
    E: EntityTrait + PagerTrait<ParentMarker = HasParent<P>>,
{
    fn parent_search(ppk: i32, rev: bool) -> Self;
    fn parent_sorted_search(ppk: i32, sort: E::SortBy, rev: bool) -> Self;
    fn from_raw(s: String, server: &Server) -> Result<Pager<E>, Error>;
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
        rev: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error>;
}

#[tonic::async_trait]
pub trait NoParentPager<E>
where
    E: EntityTrait + PagerTrait<ParentMarker = NoParent>,
{
    fn from_raw(s: String, server: &Server) -> Result<Pager<E>, Error>;
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
        rev: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error>;
}

#[tonic::async_trait]
impl<P: EntityTrait, E: EntityTrait> HasParentPager<P, E> for Pager<E>
where
    E: PagerTrait<ParentMarker = HasParent<P>>,
    P: ParentalTrait,
    P: Related<E> + Filter,
    <<P as sea_orm::EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: From<i32>,
{
    #[instrument]
    fn parent_search(ppk: i32, rev: bool) -> Self {
        Self {
            type_number: E::TYPE_NUMBER,
            sort: SearchDep::Parent(ppk),
            last_pk: None,
            last_rev: rev,
        }
    }
    #[instrument]
    fn parent_sorted_search(ppk: i32, sort: E::SortBy, rev: bool) -> Self {
        Self {
            type_number: E::TYPE_NUMBER,
            sort: SearchDep::ParentSort(ppk, sort, LastValue::default()),
            last_pk: None,
            last_rev: rev,
        }
    }
    #[instrument(skip_all, name = "pagination_deserialize", level = "trace")]
    fn from_raw(s: String, server: &Server) -> Result<Pager<E>, Error> {
        let pager = server.crypto.decode::<Pager<_>>(s).map_err(|e| {
            tracing::debug!(err=?e,"bincode_deserialize");
            Error::PaginationError("Malformated pager")
        })?;
        if pager.type_number != E::TYPE_NUMBER {
            return Err(Error::PaginationError("Pager type number mismatch"));
        }
        Ok(pager)
    }
    #[instrument(skip(self, auth))]
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
        rev: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error> {
        Self::check_bound(limit, offset)?;
        let models = match &self.sort {
            SearchDep::Text(_txt) => self.text_search_inner(limit, offset, rev, auth).await?,
            SearchDep::Column(_sort, _last_val) => {
                self.column_search_inner(limit, offset, rev, auth).await?
            }
            SearchDep::Parent(p_pk) => {
                let db = DB.get().unwrap();

                let query = P::related_read_by_id(auth, *p_pk);
                let parent = query.one(db).await?;

                if parent.is_none() {
                    return Ok(vec![]);
                }

                let mut query = parent.unwrap().find_related(E::default());

                if let Some(last) = self.last_pk {
                    let paginate = PaginatePkBuilder::default()
                        .include(self.last_rev ^ rev)
                        .rev(rev)
                        .pk(E::COL_ID)
                        .last(last)
                        .build()
                        .unwrap();
                    query = paginate.apply(query);
                } else {
                    query = order_by_bool(query, E::COL_ID, rev);
                }

                query = query.offset(offset).limit(limit);
                query
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?
            }
            SearchDep::ParentSort(p_pk, sort, last_val) => {
                let db = DB.get().unwrap();
                // TODO: select ID only
                let LastValue(inner_rev, last_val) = last_val;
                let rev = rev ^ inner_rev;

                let query = P::related_read_by_id(auth, *p_pk);
                let parent = query.one(db).await?;

                if parent.is_none() {
                    return Ok(vec![]);
                }

                let mut query = parent.unwrap().find_related(E::default());

                if let Some(last) = self.last_pk {
                    let paginate = PaginateColBuilder::default()
                        .include(self.last_rev ^ rev)
                        .rev(rev)
                        .pk(E::COL_ID)
                        .last_id(last)
                        .col(E::sort_column(sort))
                        .last_value(last_val)
                        .build()
                        .unwrap();
                    query = paginate.apply(query);
                } else {
                    query = order_by_bool(query, E::COL_ID, rev);
                }

                query = query.offset(offset).limit(limit);
                let models = query
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?;

                if let Some(model) = models.last() {
                    self.sort
                        .update_last_col(LastValue(rev, E::sort_value(model, sort)));
                }

                models
            }
        };
        if let Some(model) = models.last() {
            self.last_pk = Some(E::get_id(model));
        }
        Ok(models)
    }
}

#[tonic::async_trait]
impl<E> NoParentPager<E> for Pager<E>
where
    E: PagerTrait<ParentMarker = NoParent>,
{
    #[instrument(name = "pagination_deserialize", level = "trace", skip(server))]
    #[instrument(skip_all, name = "pagination_deserialize", level = "trace")]
    fn from_raw(s: String, server: &Server) -> Result<Pager<E>, Error> {
        let pager = server.crypto.decode::<Pager<_>>(s).map_err(|e| {
            tracing::debug!(err=?e,"bincode_deserialize");
            Error::PaginationError("Malformated pager")
        })?;
        if pager.type_number != E::TYPE_NUMBER {
            return Err(Error::PaginationError("Pager type number mismatch"));
        }
        match pager.sort {
            SearchDep::Parent(_) | SearchDep::ParentSort(_, _, _) => {
                return Err(Error::PaginationError("Pager type number mismatch"))
            }
            _ => (),
        }
        Ok(pager)
    }
    #[instrument(skip(self, auth))]
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
        rev: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error> {
        Self::check_bound(limit, offset)?;
        let models = match &self.sort {
            SearchDep::Text(_) => self.text_search_inner(limit, offset, rev, auth).await,
            SearchDep::Column(_, _) => self.column_search_inner(limit, offset, rev, auth).await,
            _ => Err(Error::Unreachable(
                "Pager<ParentMarker=NoParent> can not have parent search",
            )),
        }?;
        if let Some(model) = models.last() {
            self.last_pk = Some(E::get_id(model));
        }
        Ok(models)
    }
}

impl<E: PagerTrait> Pager<E>
where
    E: PagerTrait,
{
    #[instrument(level = "debug")]
    pub fn sort_search(sort: E::SortBy, rev: bool) -> Self {
        Self {
            type_number: E::TYPE_NUMBER,
            sort: SearchDep::Column(sort, LastValue(rev, "".to_string())),
            last_pk: None,
            last_rev: false,
        }
    }
    #[instrument(name = "pagination_deserialize", level = "trace", skip(server))]
    pub fn into_raw(self, server: &Server) -> String {
        let byte = server.crypto.encode(self);

        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            byte.unwrap(),
        )
    }
    #[instrument(level = "debug")]
    pub fn text_search(sort: String) -> Self {
        Self {
            type_number: E::TYPE_NUMBER,
            sort: SearchDep::Text(sort),
            last_pk: None,
            last_rev: false,
        }
    }
    #[instrument(skip(self, auth))]
    async fn text_search_inner(
        &mut self,
        limit: u64,
        offset: u64,
        rev: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error> {
        if let SearchDep::Text(txt) = &self.sort {
            let mut query = E::query_filter(E::find(), auth)?;
            let mut condition = E::COL_TEXT[0].like(txt.as_str());
            for col in E::COL_TEXT[1..].iter() {
                condition = condition.or(col.like(txt.as_str()));
            }
            query = query.filter(condition);

            if let Some(last) = self.last_pk {
                let paginate = PaginatePkBuilder::default()
                    .include(self.last_rev ^ rev)
                    .rev(rev)
                    .pk(E::COL_ID)
                    .last(last)
                    .build()
                    .unwrap();
                query = paginate.apply(query);
            } else {
                query = order_by_bool(query, E::COL_ID, rev);
            }
            query = query.offset(offset).limit(limit);
            Ok(query
                .limit(limit)
                .offset(offset)
                .all(DB.get().unwrap())
                .await?)
        } else {
            Err(Error::Unreachable("text_search_inner"))
        }
    }
    #[instrument(skip(self, auth))]
    async fn column_search_inner(
        &mut self,
        limit: u64,
        offset: u64,
        rev: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error> {
        if let SearchDep::Column(sort, last_val) = &self.sort {
            let mut query = E::query_filter(E::find(), auth)?;
            let LastValue(inner_rev, last_val) = last_val;
            let rev = rev ^ inner_rev;

            let col = E::sort_column(sort);

            if let Some(last) = self.last_pk {
                PaginateColBuilder::default()
                    .include(self.last_rev ^ rev)
                    .rev(rev)
                    .pk(E::COL_ID)
                    .col(col)
                    .last_id(last)
                    .last_value(last_val)
                    .build()
                    .unwrap();
            } else {
                query = order_by_bool(query, E::COL_ID, rev);
                query = order_by_bool(query, col, rev);
            }

            query = query.offset(offset).limit(limit);
            let models = query
                .limit(limit)
                .offset(offset)
                .all(DB.get().unwrap())
                .await?;

            if let Some(model) = models.last() {
                self.sort
                    .update_last_col(LastValue(rev, E::sort_value(model, sort)));
            }

            Ok(models)
        } else {
            Err(Error::Unreachable("column_search_inner"))
        }
    }

    fn check_bound(limit: u64, offset: u64) -> Result<(), Error> {
        if limit > PAGE_MAX_SIZE || offset > PAGE_MAX_OFFSET {
            Err(Error::NumberTooLarge)
        } else {
            Ok(())
        }
    }
}