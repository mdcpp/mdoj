use std::marker::PhantomData;

use ::entity::*;
use sea_orm::*;
use serde::{Deserialize, Serialize};

use crate::{grpc::backend::SortBy, init::db::DB};

use super::{auth::Auth, error::Error, filter::Filter};

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

    fn sort(select: Select<Self>, _: SortBy, _: bool) -> Select<Self> {
        select
    }
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
{
    fn parent_search(ppk: i32) -> Self;
    fn from_raw(s: String) -> Result<Pager<E>, Error>;
    fn into_raw(self) -> String;
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
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
        offset: u64,
        reverse: bool,
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
            ppk: self.ppk.unwrap_or(0),
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
            byte.unwrap(),
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
                        let sort_by = sort_by
                            .try_into()
                            .map_err(|_| Error::PaginationError("Pager reconstruction failed"))?;
                        SearchDep::Column(sort_by, reverse)
                    }
                    RawSearchDep::Parent(x) => SearchDep::Parent(x),
                };
                Ok(Pager {
                    ppk: Some(pager.ppk),
                    sort,
                    _entity: PhantomData,
                })
            }
            false => Err(Error::PaginationError("Pager type number mismatch")),
        }
    }
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
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
                let query = E::ParentMarker::related_filter(auth).await?;
                let query = query
                    .filter(E::ParentMarker::COL_ID.eq(p_pk))
                    .columns([E::ParentMarker::COL_ID])
                    .one(db)
                    .await?;

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
            .offset(offset)
            .all(DB.get().unwrap())
            .await?;

        if let Some(x) = models.last() {
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
            ppk: self.ppk.unwrap_or(0),
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
            byte.unwrap(),
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
                        let sort_by = sort_by
                            .try_into()
                            .map_err(|_| Error::PaginationError("Pager reconstruction failed"))?;
                        SearchDep::Column(sort_by, reverse)
                    }
                    RawSearchDep::Parent(_) => {
                        return Err(Error::PaginationError("Pager reconstruction failed"));
                    }
                };
                Ok(Pager {
                    ppk: Some(pager.ppk),
                    sort,
                    _entity: PhantomData,
                })
            }
            false => Err(Error::PaginationError("Pager type number mismatch")),
        }
    }
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
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
            .offset(offset)
            .all(DB.get().unwrap())
            .await?;

        if let Some(x) = models.last() {
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
impl ParentalTrait<contest::Entity> for HasParent<contest::Entity> {
    const COL_ID: contest::Column = contest::Column::Id;

    async fn related_filter(auth: &Auth) -> Result<Select<contest::Entity>, Error> {
        let db = DB.get().unwrap();

        Ok(auth.get_user(db).await?.find_related(contest::Entity))
    }
}

#[tonic::async_trait]
impl PagerTrait for problem::Entity {
    const TYPE_NUMBER: i32 = 1591223;
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
        let desc = match reverse {
            true => Order::Asc,
            false => Order::Desc,
        };
        let asc = match reverse {
            true => Order::Desc,
            false => Order::Asc,
        };
        match sort {
            SortBy::UploadDate => select.order_by(problem::Column::CreateAt, desc),
            SortBy::AcRate => select.order_by(problem::Column::AcRate, desc),
            SortBy::SubmitCount => select.order_by(problem::Column::SubmitCount, desc),
            SortBy::Difficulty => select.order_by(problem::Column::Difficulty, asc),
            _ => select,
        }
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
    async fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        problem::Entity::read_filter(select, auth)
    }
}

#[tonic::async_trait]
impl ParentalTrait<problem::Entity> for HasParent<problem::Entity> {
    const COL_ID: problem::Column = problem::Column::Id;

    async fn related_filter(auth: &Auth) -> Result<Select<problem::Entity>, Error> {
        let db = DB.get().unwrap();

        Ok(auth.get_user(db).await?.find_related(problem::Entity))
    }
}

#[tonic::async_trait]
impl PagerTrait for test::Entity {
    const TYPE_NUMBER: i32 = 78879091;
    const COL_ID: Self::Column = test::Column::Id;
    const COL_TEXT: &'static [Self::Column] = &[test::Column::Output, test::Column::Input];
    const COL_SELECT: &'static [Self::Column] = &[
        test::Column::Id,
        test::Column::UserId,
        test::Column::ProblemId,
    ];

    type ParentMarker = HasParent<problem::Entity>;

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
    async fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        test::Entity::read_filter(select, auth)
    }
}

#[tonic::async_trait]
impl PagerTrait for contest::Entity {
    const TYPE_NUMBER: i32 = 61475758;
    const COL_ID: Self::Column = contest::Column::Id;
    const COL_TEXT: &'static [Self::Column] = &[contest::Column::Title, contest::Column::Tags];
    const COL_SELECT: &'static [Self::Column] = &[
        contest::Column::Id,
        contest::Column::Title,
        contest::Column::Begin,
        contest::Column::End,
        contest::Column::Hoster,
    ];

    type ParentMarker = NoParent;

    fn sort(select: Select<Self>, sort: SortBy, reverse: bool) -> Select<Self> {
        // TODO: difficulty should be an option
        let desc = match reverse {
            true => Order::Asc,
            false => Order::Desc,
        };
        match sort {
            SortBy::UploadDate => select.order_by(contest::Column::CreateAt, desc),
            _ => select,
        }
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
    async fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        contest::Entity::read_filter(select, auth)
    }
}

#[tonic::async_trait]
impl PagerTrait for user::Entity {
    const TYPE_NUMBER: i32 = 1929833;

    const COL_ID: Self::Column = user::Column::Id;

    const COL_TEXT: &'static [Self::Column] = &[user::Column::Username];

    const COL_SELECT: &'static [Self::Column] = &[
        user::Column::Id,
        user::Column::Username,
        user::Column::Permission,
        user::Column::CreateAt,
    ];

    type ParentMarker = NoParent;

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    async fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        user::Entity::read_filter(select, auth)
    }
}
#[tonic::async_trait]
impl PagerTrait for submit::Entity {
    const TYPE_NUMBER: i32 = 539267;

    const COL_ID: Self::Column = submit::Column::Id;

    const COL_TEXT: &'static [Self::Column] = &[submit::Column::Id];

    const COL_SELECT: &'static [Self::Column] = &[
        submit::Column::Committed,
        submit::Column::Id,
        submit::Column::Time,
        submit::Column::Memory,
        submit::Column::PassCase,
        submit::Column::UploadAt,
    ];

    type ParentMarker = HasParent<problem::Entity>;

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    async fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        submit::Entity::read_filter(select, auth)
    }
}

#[tonic::async_trait]
impl PagerTrait for education::Entity {
    const TYPE_NUMBER: i32 = 183456;

    const COL_ID: Self::Column = education::Column::Id;

    const COL_TEXT: &'static [Self::Column] = &[education::Column::Title];

    const COL_SELECT: &'static [Self::Column] = &[education::Column::Id, education::Column::Title];

    type ParentMarker = HasParent<problem::Entity>;

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    async fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        education::Entity::read_filter(select, auth)
    }
}
