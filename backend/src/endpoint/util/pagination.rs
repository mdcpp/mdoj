use std::marker::PhantomData;

use ::entity::*;
use sea_orm::{*};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{grpc::backend::SortBy, init::db::DB, server::Server};

use super::{auth::Auth, error::Error, filter::Filter};

const PAGE_MAX_SIZE: u64 = 64;

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

    fn sort(select: Select<Self>, sort: &SortBy, reverse: bool) -> Select<Self> {
        let desc = match reverse {
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
    Column(i32, bool),
    Parent(i32),
}

#[derive(Serialize, Deserialize)]
struct RawPager {
    type_number: i32,
    previous: Option<String>,
    sort: RawSearchDep,
}

#[derive(Clone, Debug)]
pub enum SearchDep {
    Text(String),
    Column(SortBy, bool),
    Parent(i32),
}

#[derive(Clone, Debug)]
pub struct Pager<E: PagerTrait> {
    previous: Option<String>,
    sort: SearchDep,
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
        reverse: bool,
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
    #[instrument]
    fn parent_search(ppk: i32) -> Self {
        Self {
            previous: None,
            sort: SearchDep::Parent(ppk),
            _entity: PhantomData,
        }
    }
    #[instrument(name = "pagination_deserialize", level = "trace", skip(server))]
    fn into_raw(self, server: &Server) -> String {
        let raw = RawPager {
            type_number: E::TYPE_NUMBER,
            previous: self.previous,
            sort: match self.sort {
                SearchDep::Text(s) => RawSearchDep::Text(s),
                SearchDep::Column(sort_by, reverse) => {
                    RawSearchDep::Column(sort_by as i32, reverse)
                }
                SearchDep::Parent(x) => RawSearchDep::Parent(x),
            },
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
            RawSearchDep::Column(sort_by, reverse) => {
                let sort_by = sort_by
                    .try_into()
                    .map_err(|_| Error::PaginationError("Pager reconstruction failed"))?;
                SearchDep::Column(sort_by, reverse)
            }
            RawSearchDep::Parent(x) => SearchDep::Parent(x),
        };
        Ok(Pager {
            previous: pager.previous,
            sort,
            _entity: PhantomData,
        })
    }
    #[instrument(skip(self, auth))]
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
        reverse: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error> {
        let mut query = E::query_filter(E::find(), auth)?;

        match &self.sort {
            SearchDep::Text(txt) => {
                if let Some(previous) = &self.previous {
                    let col = E::COL_ID;
                    match reverse {
                        true => query = query.filter(ColumnTrait::lt(&col, previous)),
                        false => query = query.filter(ColumnTrait::gt(&col, previous)),
                    };
                }

                let mut condition = E::COL_TEXT[0].like(txt.as_str());
                for col in E::COL_TEXT[1..].iter() {
                    condition = condition.or(col.like(txt.as_str()));
                }

                query = query.filter(condition);

                let models = query
                    .columns(E::COL_SELECT.to_vec())
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?;

                if let Some(model) = models.last() {
                    self.previous = Some(E::get_id(model).to_string());
                }
                Ok(models)
            }
            SearchDep::Column(sort, inner_reverse) => {
                let reverse = reverse ^ inner_reverse;
                if let Some(previous) = &self.previous {
                    let col = E::sort_column(sort);
                    match reverse {
                        true => query = query.filter(ColumnTrait::lt(&col, previous)),
                        false => query = query.filter(ColumnTrait::gt(&col, previous)),
                    };
                }

                query = E::sort(query, sort, reverse);

                let models = query
                    .columns(E::COL_SELECT.to_vec())
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?;

                if let Some(model) = models.last() {
                    self.previous = Some(E::get_key_of(model, sort).to_string());
                }
                Ok(models)
            }
            SearchDep::Parent(p_pk) => {
                let db = DB.get().unwrap();
                // TODO: select ID only
                let query = E::ParentMarker::related_filter(auth).await?;
                let query = query
                    .filter(E::ParentMarker::COL_ID.eq(*p_pk))
                    .columns([E::ParentMarker::COL_ID])
                    .one(db)
                    .await?;

                if query.is_none() {
                    return Ok(vec![]);
                }

                let mut query = query.unwrap().find_related(E::default());

                query = E::query_filter(query, auth)?;

                if let Some(previous) = &self.previous {
                    let col = E::COL_ID;
                    match reverse {
                        true => query = query.filter(ColumnTrait::lt(&col, previous)),
                        false => query = query.filter(ColumnTrait::gt(&col, previous)),
                    };
                }
                let models = query
                    .columns(E::COL_SELECT.to_vec())
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?;

                if let Some(model) = models.last() {
                    self.previous = Some(E::get_id(model).to_string());
                }
                Ok(models)
            }
        }
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
            previous: self.previous,
            sort: match self.sort {
                SearchDep::Text(s) => RawSearchDep::Text(s),
                SearchDep::Column(sort_by, reverse) => {
                    RawSearchDep::Column(sort_by as i32, reverse)
                }
                SearchDep::Parent(x) => RawSearchDep::Parent(x),
            },
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
            previous: pager.previous,
            sort,
            _entity: PhantomData,
        })
    }
    #[instrument(skip(self, auth))]
    async fn fetch(
        &mut self,
        limit: u64,
        offset: u64,
        reverse: bool,
        auth: &Auth,
    ) -> Result<Vec<E::Model>, Error> {
        let mut query = E::query_filter(E::find(), auth)?;

        match &self.sort {
            SearchDep::Text(txt) => {
                if let Some(previous) = &self.previous {
                    let col = E::COL_ID;
                    match reverse {
                        true => query = query.filter(ColumnTrait::lt(&col, previous)),
                        false => query = query.filter(ColumnTrait::gt(&col, previous)),
                    };
                }

                let mut condition = E::COL_TEXT[0].like(txt.as_str());
                for col in E::COL_TEXT[1..].iter() {
                    condition = condition.or(col.like(txt.as_str()));
                }

                query = query.filter(condition);

                let models = query
                    .columns(E::COL_SELECT.to_vec())
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?;

                if let Some(model) = models.last() {
                    self.previous = Some(E::get_id(model).to_string());
                }
                Ok(models)
            }
            SearchDep::Column(sort, inner_reverse) => {
                let reverse = reverse ^ inner_reverse;
                if let Some(previous) = &self.previous {
                    let col = E::sort_column(sort);
                    match reverse {
                        true => query = query.filter(ColumnTrait::lt(&col, previous)),
                        false => query = query.filter(ColumnTrait::gt(&col, previous)),
                    };
                }

                query = E::sort(query, sort, reverse);

                let models = query
                    .columns(E::COL_SELECT.to_vec())
                    .limit(limit)
                    .offset(offset)
                    .all(DB.get().unwrap())
                    .await?;

                if let Some(model) = models.last() {
                    self.previous = Some(E::get_key_of(model, sort).to_string());
                }
                Ok(models)
            }
            SearchDep::Parent(_) => {
                unreachable!()
            }
        }
    }
}

impl<E: PagerTrait> Pager<E> {
    #[instrument(level = "debug")]
    pub fn sort_search(sort: SortBy, reverse: bool) -> Self {
        Self {
            previous: None,
            sort: SearchDep::Column(sort, reverse),
            _entity: PhantomData,
        }
    }
    #[instrument(level = "debug")]
    pub fn text_search(sort: String) -> Self {
        Self {
            previous: None,
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

    fn sort_column(sort: &SortBy) -> problem::Column {
        match sort {
            SortBy::UploadDate => problem::Column::UpdateAt,
            SortBy::CreateDate => problem::Column::CreateAt,
            SortBy::AcRate => problem::Column::AcRate,
            SortBy::SubmitCount => problem::Column::SubmitCount,
            SortBy::Difficulty => problem::Column::Difficulty,
            _ => problem::Column::Id,
        }
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String {
        match sort {
            SortBy::UploadDate => model.update_at.to_string(),
            SortBy::CreateDate => model.create_at.to_string(),
            SortBy::AcRate => model.ac_rate.to_string(),
            SortBy::SubmitCount => model.submit_count.to_string(),
            SortBy::Difficulty => model.difficulty.to_string(),
            _ => model.id.to_string(),
        }
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
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

    fn sort_column(sort: &SortBy) -> test::Column {
        match sort {
            SortBy::Score => test::Column::Score,
            _ => test::Column::Id,
        }
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String {
        match sort {
            SortBy::Score => (model.score).to_string(),
            _ => model.id.to_string(),
        }
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
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

    fn sort_column(sort: &SortBy) -> contest::Column {
        match sort {
            SortBy::CreateDate => contest::Column::CreateAt,
            SortBy::UploadDate => contest::Column::UpdateAt,
            SortBy::Begin => contest::Column::Begin,
            SortBy::End => contest::Column::End,
            _ => contest::Column::Id,
        }
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String {
        match sort {
            SortBy::CreateDate => model.create_at.to_string(),
            SortBy::UploadDate => model.update_at.to_string(),
            SortBy::Begin => model.begin.to_string(),
            SortBy::End => model.end.to_string(),
            _ => model.id.to_string(),
        }
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
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

    fn sort_column(sort: &SortBy) -> user::Column {
        match sort {
            SortBy::CreateDate => user::Column::CreateAt,
            SortBy::Score => user::Column::Score,
            _ => user::Column::Id,
        }
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String {
        match sort {
            SortBy::CreateDate => model.create_at.to_string(),
            SortBy::Score => model.score.to_string(),
            _ => model.id.to_string(),
        }
    }

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
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

    fn sort_column(sort: &SortBy) -> submit::Column {
        match sort {
            SortBy::Committed => submit::Column::Committed,
            SortBy::Score => submit::Column::Score,
            SortBy::Time => submit::Column::Time,
            SortBy::Memory => submit::Column::Memory,
            SortBy::UploadDate | SortBy::CreateDate => submit::Column::UploadAt,
            _ => submit::Column::Id,
        }
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String {
        match sort {
            SortBy::Committed => match model.committed {
                true => "1".to_string(),
                false => "0".to_string(),
            },
            SortBy::Score => model.score.to_string(),
            SortBy::Time => model.time.unwrap_or_default().to_string(),
            SortBy::Memory => model.memory.unwrap_or_default().to_string(),
            SortBy::UploadDate | SortBy::CreateDate => model.upload_at.to_string(),
            _ => model.id.to_string(),
        }
    }

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
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

    fn sort_column(_sort: &SortBy) -> education::Column {
        education::Column::Id
    }
    fn get_key_of(model: &Self::Model, _sort: &SortBy) -> String {
        model.id.to_string()
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        education::Entity::read_filter(select, auth)
    }
}
