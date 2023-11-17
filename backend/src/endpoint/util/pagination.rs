use std::marker::PhantomData;

use ::entity::*;
use sea_orm::*;
use serde::{Deserialize, Serialize};

use crate::{
    grpc::backend::SortBy,
    init::db::{self, DB},
};

use super::{auth::Auth, error::Error, filter::Filter};

#[tonic::async_trait]
pub trait ParentalTrait
where
    Self: EntityTrait,
{
    const COL_ID: Self::Column;
    async fn related_filter(auth: &Auth) -> Result<Select<Self>, Error>;
}

pub trait PagerMarker {}

pub struct NoParent;
pub struct HasParent<P: ParentalTrait> {
    _parent: PhantomData<P>,
}

impl PagerMarker for NoParent {}

impl<P: ParentalTrait> PagerMarker for HasParent<P> {}

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
    P: ParentalTrait,
{
    fn parent_search(ppk: i32) -> Self;
    fn from_raw(s: String) -> Result<Pager<E>, Error>;
    fn into_raw(self) -> String;
    async fn fetch(
        &mut self,
        limit: u64,
        reverse: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error>;
}

#[tonic::async_trait]
pub trait NoParentPager<E>
where
    E: EntityTrait + PagerTrait<ParentMarker = NoParent>,
{
    fn from_raw(s: String) -> Result<Pager<E>, Error>;
    fn into_raw(self) -> String;
    async fn fetch(
        &mut self,
        limit: u64,
        reverse: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error>;
}

#[tonic::async_trait]
impl<P: ParentalTrait, E: EntityTrait> HasParentPager<P, E> for Pager<E>
where
    E: PagerTrait<ParentMarker = HasParent<P>>,
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
    async fn fetch(
        &mut self,
        limit: u64,
        reverse: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error> {
        macro_rules! order_by_pk {
            ($src:expr,$reverse: expr) => {
                if $reverse {
                    $src = $src.order_by_asc(E::COL_ID);
                    if let Some(x) = self.ppk {
                        $src = $src.filter(E::COL_ID.gt(x));
                    }
                } else {
                    $src = $src.order_by_desc(E::COL_ID);
                    if let Some(x) = self.ppk {
                        $src = $src.filter(E::COL_ID.lt(x));
                    }
                }
            };
        }
        let query = match self.sort.clone() {
            SearchDep::Text(txt) => {
                let mut query = E::query_filter(E::find(), auth).await?;
                order_by_pk!(query, reverse);
                let mut condition = E::COL_TEXT[0].like(&txt);
                for col in E::COL_TEXT[1..].iter() {
                    condition = condition.or(col.like(&txt));
                }
                query = query.filter(condition);
                query
            }
            SearchDep::Column(sort_by, inner_reverse) => {
                let mut query = E::query_filter(E::find(), auth).await?;
                order_by_pk!(query, reverse ^ inner_reverse);
                E::sort(query, sort_by, reverse)
            }
            SearchDep::Parent(p_pk) => {
                let db = DB.get().unwrap();
                // TODO: select ID only
                let query = P::related_filter(auth).await?;
                let query = query.columns([P::COL_ID]).one(db).await?;

                if query.is_none() {
                    return Ok(vec![]);
                }

                let mut query = query.unwrap().find_related(E::default());

                query = E::query_filter(query, auth).await?;

                order_by_pk!(query, reverse);
                query
            }
        };

        let models = query
            .columns(E::COL_SELECT.to_vec())
            .limit(limit)
            .all(DB.get().unwrap())
            .await?;

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
    async fn fetch(
        &mut self,
        limit: u64,
        reverse: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error> {
        macro_rules! order_by_pk {
            ($src:expr,$reverse: expr) => {
                if $reverse {
                    $src = $src.order_by_asc(E::COL_ID);
                    if let Some(x) = self.ppk {
                        $src = $src.filter(E::COL_ID.gt(x));
                    }
                } else {
                    $src = $src.order_by_desc(E::COL_ID);
                    if let Some(x) = self.ppk {
                        $src = $src.filter(E::COL_ID.lt(x));
                    }
                }
            };
        }
        let query = match self.sort.clone() {
            SearchDep::Text(txt) => {
                let mut query = E::query_filter(E::find(), auth).await?;
                order_by_pk!(query, reverse);
                let mut condition = E::COL_TEXT[0].like(&txt);
                for col in E::COL_TEXT[1..].iter() {
                    condition = condition.or(col.like(&txt));
                }
                query = query.filter(condition);
                query
            }
            SearchDep::Column(sort_by, inner_reverse) => {
                let mut query = E::query_filter(E::find(), auth).await?;
                order_by_pk!(query, reverse ^ inner_reverse);
                E::sort(query, sort_by, reverse)
            }
            SearchDep::Parent(_) => unreachable!(),
        };

        let models = query
            .columns(E::COL_SELECT.to_vec())
            .limit(limit)
            .all(DB.get().unwrap())
            .await?;

        if let Some(x) = (&models).last() {
            self.ppk = Some(E::get_id(x));
        }

        Ok(models)
    }
}

impl<E: PagerTrait> Pager<E> {
    pub fn sort_search(sort: SortBy, reverse: bool) -> Self {
        Self {
            ppk: None,
            sort: SearchDep::Column(sort, reverse),
            _entity: PhantomData,
        }
    }
    pub fn text_search(sort: String) -> Self {
        Self {
            ppk: None,
            sort: SearchDep::Text(sort),
            _entity: PhantomData,
        }
    }
}

#[tonic::async_trait]
impl ParentalTrait for contest::Entity {
    const COL_ID: Self::Column = contest::Column::Id;

    async fn related_filter(auth: &Auth) -> Result<Select<Self>, Error> {
        let db = DB.get().unwrap();

        Ok(auth.get_user(db).await?.find_related(contest::Entity))
    }
}

#[tonic::async_trait]
impl PagerTrait for problem::Entity {
    const TYPE_NUMBER: i32 = 11223;
    const COL_ID: problem::Column = problem::Column::Id;
    const COL_TEXT: &'static [problem::Column] = &[problem::Column::Title, problem::Column::Tags];
    const COL_SELECT: &'static [problem::Column] = &[
        problem::Column::Id,
        problem::Column::Title,
        problem::Column::AcRate,
        problem::Column::SubmitCount,
        problem::Column::Difficulty,
    ];

    type ParentMarker = HasParent<contest::Entity>;

    fn sort(select: Select<Self>, sort: SortBy, reverse: bool) -> Select<Self> {
        match sort {
            SortBy::UploadDate => select.order_by_desc(problem::Column::CreateAt),
            SortBy::AcRate => select.order_by_desc(problem::Column::AcRate),
            SortBy::SubmitCount => select.order_by_desc(problem::Column::SubmitCount),
            SortBy::Difficulty => select.order_by_asc(problem::Column::Difficulty),
            _ => select,
        }
    }

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    async fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        problem::Entity::read_filter(select, auth).await
    }
}
