pub mod impls;
pub mod paginate;

use std::marker::PhantomData;

use sea_orm::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    endpoint::endpoints::paginate::{order_by_bool, PaginateColBuilder, PaginatePkBuilder},
    grpc::backend::SortBy,
    init::db::DB,
    server::Server,
};

use super::{auth::Auth, error::Error};

// TODO: add limit
const PAGE_MAX_SIZE: u64 = 64;
const PAGE_MAX_OFFSET: u64 = 256;

#[tonic::async_trait]
pub trait ParentalTrait<P>
where
    P: EntityTrait,
{
    const COL_ID: P::Column;
    async fn related_filter(auth: &Auth) -> Result<Select<P>, Error>;
}

pub trait PagerMarker {}

pub struct NoParent;
pub struct HasParent<P> {
    _parent: PhantomData<P>,
}

impl PagerMarker for NoParent {}

impl<P: EntityTrait> PagerMarker for HasParent<P> {}

/// An abstract base class for Paginatable Entity
///
/// The trait enable sort, text search, search by parent Entity
#[tonic::async_trait]
pub trait PagerTrait
where
    Self: EntityTrait,
{
    const TYPE_NUMBER: i32;
    const COL_ID: Self::Column;
    const COL_TEXT: &'static [Self::Column];
    const COL_SELECT: &'static [Self::Column];
    type ParentMarker: PagerMarker;

    fn sort(select: Select<Self>, sort: &SortBy, rev: bool) -> Select<Self> {
        let desc = match rev {
            true => Order::Asc,
            false => Order::Desc,
        };
        select.order_by(Self::sort_column(sort), desc)
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String;
    fn sort_column(sort: &SortBy) -> Self::Column;
    fn get_id(model: &Self::Model) -> i32;
    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error>;
}

#[derive(Serialize, Deserialize)]
enum RawSearchDep {
    Text(String),
    Column(i32, bool, String),
    Parent(i32),
}

#[derive(Serialize, Deserialize)]
struct RawPager {
    type_number: i32,
    sort: RawSearchDep,
    last_rev: bool,
    last_pk: Option<i32>,
}

#[derive(Clone, Debug)]
pub enum SearchDep {
    Text(String),
    Column(SortBy, bool, String),
    Parent(i32),
}

impl SearchDep {
    fn update_last_col(&mut self, data: String) {
        if let Self::Column(_a, _b, c) = self {
            *c = data;
        } else {
            unreachable!()
        }
    }
}

/// An instance of paginator itself
#[derive(Clone, Debug)]
pub struct Pager<E: PagerTrait> {
    sort: SearchDep,
    last_pk: Option<i32>,
    last_rev: bool,
    _entity: PhantomData<E>,
}

#[tonic::async_trait]
pub trait HasParentPager<P, E>
where
    E: EntityTrait + PagerTrait<ParentMarker = HasParent<P>>,
{
    fn parent_search(ppk: i32) -> Self;
    fn from_raw(s: String, server: &Server) -> Result<Pager<E>, Error>;
    fn into_raw(self, server: &Server) -> String;
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
    fn into_raw(self, server: &Server) -> String;
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
    <E as PagerTrait>::ParentMarker: ParentalTrait<P>,
    P: Related<E>,
{
    #[instrument]
    fn parent_search(ppk: i32) -> Self {
        Self {
            sort: SearchDep::Parent(ppk),
            _entity: PhantomData,
            last_pk: None,
            last_rev: false,
        }
    }
    #[instrument(name = "pagination_deserialize", level = "trace", skip(server))]
    fn into_raw(self, server: &Server) -> String {
        let raw = RawPager {
            type_number: E::TYPE_NUMBER,
            sort: match self.sort {
                SearchDep::Text(s) => RawSearchDep::Text(s),
                SearchDep::Column(sort_by, rev, last_val) => {
                    RawSearchDep::Column(sort_by as i32, rev, last_val)
                }
                SearchDep::Parent(x) => RawSearchDep::Parent(x),
            },
            last_rev: self.last_rev,
            last_pk: self.last_pk,
        };
        let byte = server.crypto.encode(raw);

        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            byte.unwrap(),
        )
    }
    #[instrument(skip_all, name = "pagination_deserialize", level = "trace")]
    fn from_raw(s: String, server: &Server) -> Result<Pager<E>, Error> {
        let byte = base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, s)
            .map_err(|e| {
                tracing::trace!(err=?e,"base64_deserialize");
                Error::PaginationError("Not base64")
            })?;
        let pager = server.crypto.decode::<RawPager>(byte).map_err(|e| {
            tracing::debug!(err=?e,"bincode_deserialize");
            Error::PaginationError("Malformated pager")
        })?;
        if pager.type_number != E::TYPE_NUMBER {
            return Err(Error::PaginationError("Pager type number mismatch"));
        }
        let sort = match pager.sort {
            RawSearchDep::Text(x) => SearchDep::Text(x),
            RawSearchDep::Column(sort_by, rev, last_val) => {
                let sort_by = sort_by
                    .try_into()
                    .map_err(|_| Error::PaginationError("Pager reconstruction failed"))?;
                SearchDep::Column(sort_by, rev, last_val)
            }
            RawSearchDep::Parent(x) => SearchDep::Parent(x),
        };
        Ok(Pager {
            sort,
            _entity: PhantomData,
            last_pk: pager.last_pk,
            last_rev: pager.last_rev,
        })
    }
    #[instrument(skip(self, auth))]
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
        rev: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error> {
        let models = match &self.sort {
            SearchDep::Text(txt) => {
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
                query
                    .columns(E::COL_SELECT.to_vec())
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?
            }
            SearchDep::Column(sort, inner_rev, last_val) => {
                let mut query = E::query_filter(E::find(), auth)?;
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
                    .columns(E::COL_SELECT.to_vec())
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?;

                if let Some(model) = models.last() {
                    self.sort.update_last_col(E::get_key_of(model, sort));
                }

                models
            }
            SearchDep::Parent(p_pk) => {
                let db = DB.get().unwrap();
                // TODO: select ID only
                let query = E::ParentMarker::related_filter(auth).await?;
                let parent = query
                    .filter(E::ParentMarker::COL_ID.eq(*p_pk))
                    .columns([E::ParentMarker::COL_ID])
                    .one(db)
                    .await?;

                if parent.is_none() {
                    return Ok(vec![]);
                }

                let mut query = parent.unwrap().find_related(E::default());

                query = E::query_filter(query, auth)?;

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
                    .columns(E::COL_SELECT.to_vec())
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?
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
    fn into_raw(self, server: &Server) -> String {
        let raw = RawPager {
            type_number: E::TYPE_NUMBER,
            sort: match self.sort {
                SearchDep::Text(s) => RawSearchDep::Text(s),
                SearchDep::Column(sort_by, rev, last_val) => {
                    RawSearchDep::Column(sort_by as i32, rev, last_val)
                }
                SearchDep::Parent(x) => RawSearchDep::Parent(x),
            },
            last_pk: self.last_pk,
            last_rev: self.last_rev,
        };
        let byte = server.crypto.encode(raw);

        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            byte.unwrap(),
        )
    }
    #[instrument(skip_all, name = "pagination_deserialize", level = "trace")]
    fn from_raw(s: String, server: &Server) -> Result<Pager<E>, Error> {
        let byte = base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, s)
            .map_err(|e| {
                tracing::trace!(err=?e,"base64_deserialize");
                Error::PaginationError("Not base64")
            })?;
        let pager = server.crypto.decode::<RawPager>(byte).map_err(|e| {
            tracing::debug!(err=?e,"bincode_deserialize");
            Error::PaginationError("Malformated pager")
        })?;
        if pager.type_number != E::TYPE_NUMBER {
            return Err(Error::PaginationError("Pager type number mismatch"));
        }
        let sort = match pager.sort {
            RawSearchDep::Text(x) => SearchDep::Text(x),
            RawSearchDep::Column(sort_by, rev, last_val) => {
                let sort_by = sort_by
                    .try_into()
                    .map_err(|_| Error::PaginationError("Pager reconstruction failed"))?;
                SearchDep::Column(sort_by, rev, last_val)
            }
            RawSearchDep::Parent(_) => {
                return Err(Error::PaginationError("Pager reconstruction failed"));
            }
        };
        Ok(Pager {
            sort,
            _entity: PhantomData,
            last_pk: pager.last_pk,
            last_rev: pager.last_rev,
        })
    }
    #[instrument(skip(self, auth))]
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
        rev: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error> {
        let models = match &self.sort {
            SearchDep::Text(txt) => {
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
                query
                    .columns(E::COL_SELECT.to_vec())
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?
            }
            SearchDep::Column(sort, inner_rev, last_val) => {
                let mut query = E::query_filter(E::find(), auth)?;
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
                    .columns(E::COL_SELECT.to_vec())
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?;

                if let Some(model) = models.last() {
                    self.sort.update_last_col(E::get_key_of(model, sort));
                }

                models
            }
            SearchDep::Parent(_p_pk) => {
                unreachable!()
            }
        };
        if let Some(model) = models.last() {
            self.last_pk = Some(E::get_id(model));
        }
        Ok(models)
    }
}

impl<E: PagerTrait> Pager<E> {
    #[instrument(level = "debug")]
    pub fn sort_search(sort: SortBy, rev: bool) -> Self {
        Self {
            sort: SearchDep::Column(sort, rev, "".to_string()),
            _entity: PhantomData,
            last_pk: None,
            last_rev: false,
        }
    }
    #[instrument(level = "debug")]
    pub fn text_search(sort: String) -> Self {
        Self {
            sort: SearchDep::Text(sort),
            _entity: PhantomData,
            last_pk: None,
            last_rev: false,
        }
    }
}
