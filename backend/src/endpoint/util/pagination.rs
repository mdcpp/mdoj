use std::marker::PhantomData;

use ::entity::*;
use sea_orm::*;
use serde::{Deserialize, Serialize};

use crate::{endpoint::tools::DB, grpc::backend::SortBy};

pub trait PagerTrait
where
    Self: EntityTrait,
{
    const COL_ID: Self::Column;

    fn sort(select: Select<Self>, sort: SortBy, reverse: bool) -> Select<Self>;
    fn get_id(model: &Self::Model) -> i32;
}

#[derive(Serialize, Deserialize)]
struct RawPager {
    ppk: i32,
    sort: i32,
    reverse: bool,
}

#[derive(Clone)]
pub struct Pager<E: PagerTrait> {
    ppk: Option<i32>,
    sort: SortBy,
    reverse: bool,
    _entity:PhantomData<E>
}

impl<E: PagerTrait> Pager<E>
where
    Value: From<<E as EntityTrait>::PrimaryKey>,
{
    pub fn new(sort: SortBy, reverse: bool) -> Self {
        Self {
            ppk: None,
            sort,
            reverse,
            _entity:PhantomData
        }
    }
    pub async fn fetch(&mut self, limit: u64) -> Result<Vec<E::Model>, sea_orm::DbErr> {
        let query: Select<E> = self.clone().into();

        let models = query.limit(limit).all(DB.get().unwrap()).await?;

        if let Some(x) = (&models).last() {
            self.ppk = Some(E::get_id(x));
        }

        Ok(models)
    }
    pub fn into_raw(self) -> String
    where
        i32: From<<E as sea_orm::EntityTrait>::PrimaryKey>,
    {
        let raw = RawPager {
            ppk: self.ppk.map(|x| x.into()).unwrap_or(0),
            sort: self.sort.into(),
            reverse: self.reverse,
        };
        let byte = bincode::serialize(&raw);

        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            &byte.unwrap(),
        )
    }
    pub fn from_raw(s: String) -> Option<String> {
        let byte =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, s).ok()?;
        bincode::deserialize(&byte).ok()
    }
}

impl<E: PagerTrait> Into<Select<E>> for Pager<E>
where
    Value: From<<E as EntityTrait>::PrimaryKey>,
{
    fn into(self) -> Select<E> {
        let mut query = E::sort(E::find(), self.sort, self.reverse);
        if self.reverse {
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
        query
    }
}

impl PagerTrait for problem::Entity{
    const COL_ID: problem::Column = problem::Column::Id;

    fn sort(select: Select<Self>, sort: SortBy, reverse: bool) -> Select<Self> {
        let col=match sort{
            SortBy::UploadDate => problem::Column::CreateAt,
            SortBy::AcRate => problem::Column::AcRate,
            SortBy::SubmitCount => problem::Column::SubmitCount,
            SortBy::Difficulty => problem::Column::Difficulty,
            _ =>{
                return select;
            }
        };
        if reverse{
            select.order_by_desc(col)
        }else{
            select.order_by_asc(col)
        }
    }

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
}
