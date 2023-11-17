use std::marker::PhantomData;

use ::entity::*;
use sea_orm::*;
use serde::{Deserialize, Serialize};

use crate::{grpc::backend::SortBy, init::db::DB};

use super::{auth::Auth, error::Error};

pub trait PagerMarker {}

pub struct NoParent;
pub struct HasParent<P: EntityTrait> {
    _parent: PhantomData<P>,
}

impl PagerMarker for NoParent {}

impl<P: EntityTrait> PagerMarker for HasParent<P> {}

#[tonic::async_trait]
pub trait PagerTrait
where
    Self: EntityTrait,
{
    const TYPE_NUMBER: i32;
    const COL_ID: Self::Column;
    const COL_TEXT: &'static [Self::Column];
    type ParentMarker: PagerMarker;
    // type ParentEntity: EntityTrait;

    fn sort(select: Select<Self>, sort: SortBy, reverse: bool) -> Select<Self>;
    fn get_id(model: &Self::Model) -> i32;
    async fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error>;
}

#[derive(Serialize, Deserialize)]
enum RawSearchDep {
    Text(String),
    Column(i32, bool),
    Parent(i32),
}

#[derive(Serialize, Deserialize)]
struct RawPager {
    type_number: i32,
    ppk: i32,
    sort: RawSearchDep,
}

#[derive(Clone)]
pub enum SearchDep {
    Text(String),
    Column(SortBy, bool),
    Parent(i32),
}

#[derive(Clone)]
pub struct Pager<E: PagerTrait> {
    ppk: Option<i32>,
    sort: SearchDep,
    _entity: PhantomData<E>,
}

#[tonic::async_trait]
pub trait HasParentPager<P, E>
where
    E: EntityTrait + PagerTrait<ParentMarker = HasParent<P>>,
    P: EntityTrait,
{
    fn parent_search(ppk: i32) -> Self;
    fn from_raw(s: String) -> Result<Pager<E>, Error>;
    fn into_raw(self) -> String;
    async fn fetch(&mut self, limit: u64, auth: &Auth) -> Result<Vec<E::Model>, Error>;
}

#[tonic::async_trait]
pub trait NoParentPager<E>
where
    E: EntityTrait + PagerTrait<ParentMarker = NoParent>,
{
    fn from_raw(s: String) -> Result<Pager<E>, Error>;
    fn into_raw(self) -> String;
    async fn fetch(&mut self, limit: u64, auth: &Auth) -> Result<Vec<E::Model>, Error>;
}

#[tonic::async_trait]
impl<P: EntityTrait, E: EntityTrait> HasParentPager<P, E> for Pager<E>
where
    E: PagerTrait<ParentMarker = HasParent<P>>,
    Value: From<<E as EntityTrait>::PrimaryKey>,
    i32: From<<E as sea_orm::EntityTrait>::PrimaryKey>,
    <P::PrimaryKey as PrimaryKeyTrait>::ValueType: From<i32>,
    P: Related<E>,
{
    fn parent_search(ppk: i32) -> Self {
        Self {
            ppk: None,
            sort: SearchDep::Parent(ppk),
            _entity: PhantomData,
        }
    }
    fn into_raw(self) -> String {
        let raw = RawPager {
            type_number: E::TYPE_NUMBER,
            ppk: self.ppk.map(|x| x.into()).unwrap_or(0),
            sort: match self.sort {
                SearchDep::Text(s) => RawSearchDep::Text(s),
                SearchDep::Column(sort_by, reverse) => {
                    RawSearchDep::Column(sort_by as i32, reverse)
                }
                SearchDep::Parent(x) => RawSearchDep::Parent(x),
            },
        };
        let byte = bincode::serialize(&raw);

        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            &byte.unwrap(),
        )
    }
    fn from_raw(s: String) -> Result<Pager<E>, Error> {
        let byte = base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, s)
            .map_err(|e| {
                log::trace!("Pager base64 deserialize error: {}", e);
                Error::PaginationError("Invaild pager")
            })?;
        let pager = bincode::deserialize::<RawPager>(&byte).map_err(|e| {
            log::trace!("Pager bincode deserialize error: {}", e);
            Error::PaginationError("Invaild pager")
        })?;
        match pager.type_number == E::TYPE_NUMBER {
            true => {
                let sort = match pager.sort {
                    RawSearchDep::Text(x) => SearchDep::Text(x),
                    RawSearchDep::Column(sort_by, reverse) => {
                        let sort_by = SortBy::from_i32(sort_by)
                            .ok_or(Error::PaginationError("Pager reconstruction failed"))?;
                        SearchDep::Column(sort_by, reverse)
                    }
                    RawSearchDep::Parent(x) => SearchDep::Parent(x),
                };
                return Ok(Pager {
                    ppk: Some(pager.ppk),
                    sort,
                    _entity: PhantomData,
                });
            }
            false => Err(Error::PaginationError("Pager type number mismatch")),
        }
    }
    async fn fetch(&mut self, limit: u64, auth: &Auth) -> Result<Vec<E::Model>, Error> {
        let query = match self.sort.clone() {
            SearchDep::Text(txt) => {
                let mut query = E::query_filter(E::find(), auth).await?;
                let mut condition = E::COL_TEXT[0].like(&txt);
                for col in E::COL_TEXT[1..].iter() {
                    condition = condition.or(col.like(&txt));
                }
                query = query.order_by_asc(E::COL_ID);
                if let Some(x) = self.ppk {
                    query = query.filter(E::COL_ID.gt(x));
                }
                query = query.filter(condition);
                query
            }
            SearchDep::Column(sort_by, reverse) => {
                let mut query = E::query_filter(E::find(), auth).await?;
                if reverse {
                    query = query.order_by_asc(E::COL_ID);
                    if let Some(x) = self.ppk {
                        query = query.filter(E::COL_ID.gt(x));
                    }
                } else {
                    query = query.order_by_desc(E::COL_ID);
                    if let Some(x) = self.ppk {
                        query = query.filter(E::COL_ID.lt(x));
                    }
                }
                E::sort(query, sort_by, reverse)
            }
            SearchDep::Parent(p_pk) => {
                let db = DB.get().unwrap();
                // TODO: select ID only
                let query = P::find_by_id(p_pk).one(db).await?;

                if query.is_none() {
                    return Ok(vec![]);
                }

                let query = query.unwrap().find_related(E::default());

                let query = E::query_filter(query, auth).await?;

                let mut query = query.order_by_asc(E::COL_ID);
                if let Some(x) = self.ppk {
                    query = query.filter(E::COL_ID.gt(x));
                }
                query
            }
        };

        let models = query.limit(limit).all(DB.get().unwrap()).await?;

        if let Some(x) = (&models).last() {
            self.ppk = Some(E::get_id(x));
        }

        Ok(models)
    }
}

#[tonic::async_trait]
impl<E> NoParentPager<E> for Pager<E>
where
    E: PagerTrait<ParentMarker = NoParent>,
    Value: From<<E as EntityTrait>::PrimaryKey>,
    i32: From<<E as sea_orm::EntityTrait>::PrimaryKey>,
{
    fn into_raw(self) -> String {
        let raw = RawPager {
            type_number: E::TYPE_NUMBER,
            ppk: self.ppk.map(|x| x.into()).unwrap_or(0),
            sort: match self.sort {
                SearchDep::Text(s) => RawSearchDep::Text(s),
                SearchDep::Column(sort_by, reverse) => {
                    RawSearchDep::Column(sort_by as i32, reverse)
                }
                SearchDep::Parent(x) => RawSearchDep::Parent(x),
            },
        };
        let byte = bincode::serialize(&raw);

        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            &byte.unwrap(),
        )
    }
    fn from_raw(s: String) -> Result<Pager<E>, Error> {
        let byte = base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, s)
            .map_err(|e| {
                log::trace!("Pager base64 deserialize error: {}", e);
                Error::PaginationError("Invaild pager")
            })?;
        let pager = bincode::deserialize::<RawPager>(&byte).map_err(|e| {
            log::trace!("Pager bincode deserialize error: {}", e);
            Error::PaginationError("Invaild pager")
        })?;
        match pager.type_number == E::TYPE_NUMBER {
            true => {
                let sort = match pager.sort {
                    RawSearchDep::Text(x) => SearchDep::Text(x),
                    RawSearchDep::Column(sort_by, reverse) => {
                        let sort_by = SortBy::from_i32(sort_by)
                            .ok_or(Error::PaginationError("Pager reconstruction failed"))?;
                        SearchDep::Column(sort_by, reverse)
                    }
                    RawSearchDep::Parent(x) => {
                        return Err(Error::PaginationError("Pager reconstruction failed"));
                    }
                };
                return Ok(Pager {
                    ppk: Some(pager.ppk),
                    sort,
                    _entity: PhantomData,
                });
            }
            false => Err(Error::PaginationError("Pager type number mismatch")),
        }
    }
    async fn fetch(&mut self, limit: u64, auth: &Auth) -> Result<Vec<E::Model>, Error> {
        let query = match self.sort.clone() {
            SearchDep::Text(txt) => {
                let mut query = E::query_filter(E::find(), auth).await?;
                let mut condition = E::COL_TEXT[0].like(&txt);
                for col in E::COL_TEXT[1..].iter() {
                    condition = condition.or(col.like(&txt));
                }
                query = query.order_by_asc(E::COL_ID);
                if let Some(x) = self.ppk {
                    query = query.filter(E::COL_ID.gt(x));
                }
                query = query.filter(condition);
                query
            }
            SearchDep::Column(sort_by, reverse) => {
                let mut query = E::query_filter(E::find(), auth).await?;
                if reverse {
                    query = query.order_by_asc(E::COL_ID);
                    if let Some(x) = self.ppk {
                        query = query.filter(E::COL_ID.gt(x));
                    }
                } else {
                    query = query.order_by_desc(E::COL_ID);
                    if let Some(x) = self.ppk {
                        query = query.filter(E::COL_ID.lt(x));
                    }
                }
                E::sort(query, sort_by, reverse)
            }
            SearchDep::Parent(p_pk) => {
                unreachable!();
            }
        };

        let models = query.limit(limit).all(DB.get().unwrap()).await?;

        if let Some(x) = (&models).last() {
            self.ppk = Some(E::get_id(x));
        }

        Ok(models)
    }
}

impl<E: PagerTrait> Pager<E>
where
    Value: From<<E as EntityTrait>::PrimaryKey>,
{
    pub fn sort_search(sort: SortBy, reverse: bool) -> Self {
        Self {
            ppk: None,
            sort: SearchDep::Column(sort, reverse),
            _entity: PhantomData,
        }
    }
    pub fn text_search(sort: String, reverse: bool) -> Self {
        Self {
            ppk: None,
            sort: SearchDep::Text(sort),
            _entity: PhantomData,
        }
    }
}

// impl PagerTrait for problem::Entity {
//     const TYPE_NUMBER: i32 = const_random::const_random!(i32);
//     const COL_ID: problem::Column = problem::Column::Id;
//     const COL_TEXT: &'static [problem::Column] =
//         &[problem::Column::Title, problem::Column::Content];

//     fn sort(select: Select<Self>, sort: SortBy, reverse: bool) -> Select<Self> {
//         let col = match sort {
//             SortBy::UploadDate => problem::Column::CreateAt,
//             SortBy::AcRate => problem::Column::AcRate,
//             SortBy::SubmitCount => problem::Column::SubmitCount,
//             SortBy::Difficulty => problem::Column::Difficulty,
//             _ => {
//                 return select;
//             }
//         };
//         if reverse {
//             select.order_by_desc(col)
//         } else {
//             select.order_by_asc(col)
//         }
//     }
//     fn get_id(model: &Self::Model) -> i32 {
//         model.id
//     }
// }
