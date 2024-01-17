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

pub(self) mod paginate;

use entity::DebugName;
use paginate::*;
use tonic::async_trait;

use std::{io::Cursor, marker::PhantomData};

use sea_orm::{ColumnTrait, EntityTrait, QuerySelect, Select};

use super::error::Error;

pub trait Dump
where
    Self: Sized,
{
    fn serialize(self) -> Vec<u8>;
    fn deserialize(raw: &[u8]) -> Result<Self, Error>;
}

impl Dump for () {
    fn serialize(self) -> Vec<u8> {
        Default::default()
    }

    fn deserialize(raw: &[u8]) -> Result<Self, Error> {
        Ok(())
    }
}

/// indicate foreign object is ready for page source
///
/// In `Education` example, we expect ::entity::education::Entity
/// to implement it
pub trait PagerSource<D>
where
    Self: Send,
{
    type Id: ColumnTrait + Default;
    type Entity: EntityTrait + DebugName;
    /// filter reconstruction
    fn filter(data: &D) -> Select<Self::Entity>;
}

/// indicate foreign object is ready for page reflect
///
/// In `Education` example, we expect ::entity::education::PartialEducation
/// to implement it
#[async_trait]
pub trait PagerReflect<E: EntityTrait>
where
    Self: Sized + Send,
{
    /// get id of primary key
    fn get_id(&self) -> i32;
    async fn all(query: Select<E>) -> Result<Vec<Self>, Error>;
}

#[async_trait]
pub trait Pager<D: Send>
where
    Self: Sized + Dump,
{
    type Source: PagerSource<D>;
    type Reflect: PagerReflect<<Self::Source as PagerSource<D>>::Entity> + Send;
    async fn fetch(
        self,
        size: u64,
        offset: u64,
        rel_dir: bool,
    ) -> Result<(Self, Vec<Self::Reflect>), Error>;
    async fn new_fetch(
        data: D,
        size: u64,
        offset: u64,
        abs_dir: bool,
    ) -> Result<(Self, Vec<Self::Reflect>), Error>;
}

/// compact primary key pager
pub struct PkPager<D, S: PagerSource<D>, R: PagerReflect<S::Entity>, const T: u8> {
    /// last primary
    last_id: i32,
    /// last direction(relative)
    last_direction: bool,
    /// original direction
    direction: bool,
    /// data
    data: D,
    source: PhantomData<S>,
    reflect: PhantomData<R>,
}

impl<D: Dump, S: PagerSource<D>, R: PagerReflect<S::Entity>, const T: u8> Dump
    for PkPager<D, S, R, T>
{
    fn serialize(self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(2);
        let mut type_number = T & 0xfc;
        if self.last_direction {
            type_number |= 0x2;
        }
        if self.direction {
            type_number |= 0x1;
        }
        buffer.push(type_number.to_be());

        let mut value = self.last_id as i64;
        loop {
            let mut tmp: i16 = (value & 0b0111_1111) as i16;
            value >>= 7;
            if value != 0 {
                tmp |= 0b1000_0000;
            }
            buffer.push((tmp as i8).to_be_bytes()[0]);
            if value == 0 {
                break;
            }
        }

        [buffer, self.data.serialize()].concat()
    }
    fn deserialize(raw: &[u8]) -> Result<Self, Error> {
        let mut c = raw.iter();
        let notation = *c.next().ok_or(Error::PaginationError("Not enough byte"))?;
        if ((notation ^ T) & 0xfc) != 0 {
            return Err(Error::PaginationError("type mismatched"));
        }

        let last_direction = (notation & 0x2) != 0;
        let direction = (notation & 0x1) != 0;

        let mut last_id: i32 = 0;
        for num_read in 0..5 {
            let read = *c.next().ok_or(Error::PaginationError("Not enough byte"))? as i32;
            let value = read & 0b0111_1111;
            last_id |= value << (7 * num_read);
            if (read & 0b1000_0000) == 0 {
                break;
            }
        }
        let data = D::deserialize(c.as_slice())?;

        Ok(Self {
            last_id,
            source: PhantomData,
            reflect: PhantomData,
            data,
            last_direction,
            direction,
        })
    }
}

#[async_trait]
impl<D: Send + Dump, const T: u8, S: PagerSource<D>, R: PagerReflect<S::Entity>> Pager<D>
    for PkPager<D, S, R, T>
{
    type Source = S;
    type Reflect = R;

    async fn fetch(
        mut self,
        size: u64,
        offset: u64,
        rel_dir: bool,
    ) -> Result<(Self, Vec<Self::Reflect>), Error> {
        let paginator = PaginatePkBuilder::default()
            .pk(<S as PagerSource<D>>::Id::default())
            .include(self.last_direction ^ rel_dir)
            .rev(self.direction ^ rel_dir)
            .last_pk(self.last_id)
            .build()
            .unwrap();

        self.last_direction = rel_dir;

        let query = paginator
            .apply(S::filter(&self.data))
            .limit(size)
            .offset(offset);
        let models = R::all(query).await?;

        if let Some(model) = models.last() {
            self.last_id = R::get_id(&model);
            return Ok((self, models));
        }

        Err(Error::NotInDB(S::Entity::DEBUG_NAME))
    }
    async fn new_fetch(
        data: D,
        size: u64,
        offset: u64,
        abs_dir: bool,
    ) -> Result<(Self, Vec<Self::Reflect>), Error> {
        // FIXME: this is consider some kind of hack, only work with sqlite
        let last_id = match abs_dir {
            true => i32::MAX,
            false => i32::MIN,
        };
        let pager = Self {
            last_id,
            last_direction: true,
            direction: abs_dir,
            data,
            source: PhantomData,
            reflect: PhantomData,
        };
        pager.fetch(size, offset, abs_dir).await
    }
}

pub trait PagerSortSource<D, R>
where
    Self: PagerSource<D>,
{
    /// get sort column
    fn sort_col(data: &D) -> impl ColumnTrait;
    fn get_val(data: &D) -> impl Into<sea_orm::Value> + Clone;
    /// save last value in column
    fn save_val(data: &mut D, model: &R) -> impl ColumnTrait;
}

/// compact column pager
pub struct ColPager<D, S: PagerSortSource<D, R>, R: PagerReflect<S::Entity>, const T: u8> {
    /// last primary
    last_id: i32,
    /// last direction(relative)
    last_direction: bool,
    /// original direction
    direction: bool,
    data: D,
    source: PhantomData<S>,
    reflect: PhantomData<R>,
}

impl<D: Dump, S: PagerSortSource<D, R>, R: PagerReflect<S::Entity>, const T: u8> Dump
    for ColPager<D, S, R, T>
{
    fn serialize(self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(2);
        let mut type_number = T & 0xfc;
        if self.last_direction {
            type_number |= 0x2;
        }
        if self.direction {
            type_number |= 0x1;
        }
        buffer.push(type_number.to_be());

        let mut value = self.last_id as i64;
        loop {
            let mut tmp: i16 = (value & 0b0111_1111) as i16;
            value >>= 7;
            if value != 0 {
                tmp |= 0b1000_0000;
            }
            buffer.push((tmp as i8).to_be_bytes()[0]);
            if value == 0 {
                break;
            }
        }

        [buffer, self.data.serialize()].concat()
    }
    fn deserialize(raw: &[u8]) -> Result<Self, Error> {
        let mut c = raw.iter();
        let notation = *c.next().ok_or(Error::PaginationError("Not enough byte"))?;
        if ((notation ^ T) & 0xfc) != 0 {
            return Err(Error::PaginationError("type mismatched"));
        }

        let last_direction = (notation & 0x2) != 0;
        let direction = (notation & 0x1) != 0;

        let mut last_id: i32 = 0;
        for num_read in 0..5 {
            let read = *c.next().ok_or(Error::PaginationError("Not enough byte"))? as i32;
            let value = read & 0b0111_1111;
            last_id |= value << (7 * num_read);
            if (read & 0b1000_0000) == 0 {
                break;
            }
        }
        let data = D::deserialize(c.as_slice())?;

        Ok(Self {
            last_id,
            source: PhantomData,
            reflect: PhantomData,
            data,
            last_direction,
            direction,
        })
    }
}

#[async_trait]
impl<D: Send + Dump, S: PagerSortSource<D, R>, R: PagerReflect<S::Entity>, const T: u8> Pager<D>
    for ColPager<D, S, R, T>
{
    type Source = S;
    type Reflect = R;

    async fn fetch(
        mut self,
        size: u64,
        offset: u64,
        rel_dir: bool,
    ) -> Result<(Self, Vec<Self::Reflect>), Error> {
        let col = S::sort_col(&self.data);
        let val = S::get_val(&self.data);

        let paginator = PaginateColBuilder::default()
            .pk(<S as PagerSource<D>>::Id::default())
            .include(self.last_direction ^ rel_dir)
            .rev(self.direction ^ rel_dir)
            .col(col)
            .last_value(val)
            .last_pk(self.last_id)
            .build()
            .unwrap();

        self.last_direction = rel_dir;

        let query = paginator
            .apply(S::filter(&self.data))
            .limit(size)
            .offset(offset);
        let models = R::all(query).await?;

        if let Some(model) = models.last() {
            self.last_id = R::get_id(&model);
            return Ok((self, models));
        }

        Err(Error::NotInDB(S::Entity::DEBUG_NAME))
    }
    async fn new_fetch(
        data: D,
        size: u64,
        offset: u64,
        abs_dir: bool,
    ) -> Result<(Self, Vec<Self::Reflect>), Error> {
        let last_id = match abs_dir {
            true => i32::MAX,
            false => i32::MIN,
        };
        let pager = Self {
            last_id,
            last_direction: true,
            direction: abs_dir,
            data,
            source: PhantomData,
            reflect: PhantomData,
        };
        pager.fetch(size, offset, abs_dir).await
    }
}
