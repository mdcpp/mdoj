use std::marker::PhantomData;

use ::entity::*;
use sea_orm::*;
use serde::{Deserialize, Serialize};

use crate::{grpc::backend::SortBy, init::db::DB};

pub trait PagerTrait
where
    Self: EntityTrait,
{
    const TYPE_NUMBER: i32;
    const COL_ID: Self::Column;
    const COL_TEXT: &'static [Self::Column];

    fn sort(select: Select<Self>, sort: SortBy, reverse: bool) -> Select<Self>;
    fn get_id(model: &Self::Model) -> i32;
}

#[derive(Serialize, Deserialize)]
enum RawSearchDep {
    Text(String),
    Column(i32, bool),
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
}

#[derive(Clone)]
pub struct Pager<E: PagerTrait> {
    ppk: Option<i32>,
    sort: SearchDep,
    _entity: PhantomData<E>,
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
    pub async fn fetch(
        &mut self,
        limit: u64,
        filter: Select<E>,
    ) -> Result<Vec<E::Model>, sea_orm::DbErr> {
        let query: Select<E> = self.clone().into_query(filter);

        let models = query.limit(limit).all(DB.get().unwrap()).await?;

        if let Some(x) = (&models).last() {
            self.ppk = Some(E::get_id(x));
        }

        Ok(models)
    }
    fn into_query(self, mut query: Select<E>) -> Select<E> {
        match self.sort {
            SearchDep::Text(txt) => {
                let mut condition = E::COL_TEXT[0].like(&txt);
                for col in E::COL_TEXT[1..].iter() {
                    condition = condition.or(col.like(&txt));
                }
                query = query.order_by_asc(E::COL_ID);
                if let Some(x) = self.ppk {
                    query = query.filter(E::COL_ID.gt(x));
                }
                query = query.filter(condition)
            }
            SearchDep::Column(sort_by, reverse) => {
                query = E::sort(query, sort_by, reverse);
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
            }
        }
        query
    }
    pub fn into_raw(self) -> String
    where
        i32: From<<E as sea_orm::EntityTrait>::PrimaryKey>,
    {
        let raw = RawPager {
            type_number: E::TYPE_NUMBER,
            ppk: self.ppk.map(|x| x.into()).unwrap_or(0),
            sort: match self.sort {
                SearchDep::Text(s) => RawSearchDep::Text(s),
                SearchDep::Column(sort_by, reverse) => {
                    RawSearchDep::Column(sort_by as i32, reverse)
                }
            },
        };
        let byte = bincode::serialize(&raw);

        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            &byte.unwrap(),
        )
    }
    pub fn from_raw(s: String) -> Option<Pager<E>> {
        let byte =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, s).ok()?;
        let pager = bincode::deserialize::<RawPager>(&byte).ok()?;
        if pager.type_number == E::TYPE_NUMBER {
            let sort = match pager.sort {
                RawSearchDep::Text(x) => SearchDep::Text(x),
                RawSearchDep::Column(sort_by, reverse) => {
                    let sort_by = SortBy::from_i32(sort_by)?;
                    SearchDep::Column(sort_by, reverse)
                }
            };
            return Some(Pager {
                ppk: Some(pager.ppk),
                sort,
                _entity: PhantomData,
            });
        }
        None
    }
}

impl PagerTrait for problem::Entity {
    const TYPE_NUMBER: i32 = const_random::const_random!(i32);
    const COL_ID: problem::Column = problem::Column::Id;
    const COL_TEXT: &'static [problem::Column] =
        &[problem::Column::Title, problem::Column::Content];

    fn sort(select: Select<Self>, sort: SortBy, reverse: bool) -> Select<Self> {
        let col = match sort {
            SortBy::UploadDate => problem::Column::CreateAt,
            SortBy::AcRate => problem::Column::AcRate,
            SortBy::SubmitCount => problem::Column::SubmitCount,
            SortBy::Difficulty => problem::Column::Difficulty,
            _ => {
                return select;
            }
        };
        if reverse {
            select.order_by_desc(col)
        } else {
            select.order_by_asc(col)
        }
    }

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
}
