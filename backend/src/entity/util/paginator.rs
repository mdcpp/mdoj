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

use crate::{entity::DebugName, util::auth::Auth};
use tonic::async_trait;

use std::marker::PhantomData;

use sea_orm::{ColumnTrait, EntityTrait, QuerySelect, Select};

use crate::util::error::Error;

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

    fn deserialize(_raw: &[u8]) -> Result<Self, Error> {
        Ok(())
    }
}

/// indicate foreign object is ready for page source
///
/// In `Education` example, we expect ::entity::education::Entity
/// to implement it
pub trait PagerSource
where
    Self: Send,
{
    const ID: <Self::Entity as EntityTrait>::Column;
    type Entity: EntityTrait + DebugName;
    type Data: Send + Sized + Dump;
    const TYPE_NUMBER: u8;
    /// filter reconstruction
    fn filter(auth: &Auth, data: &Self::Data) -> Select<Self::Entity>;
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
pub trait Pager
where
    Self: Sized + Dump,
{
    type Source: PagerSource;
    type Reflect: PagerReflect<<Self::Source as PagerSource>::Entity> + Send;
    async fn fetch(
        self,
        auth: &Auth,
        size: u64,
        offset: u64,
        rel_dir: bool,
    ) -> Result<(Self, Vec<Self::Reflect>), Error>;
    async fn new_fetch(
        data: <Self::Source as PagerSource>::Data,
        auth: &Auth,
        size: u64,
        offset: u64,
        abs_dir: bool,
    ) -> Result<(Self, Vec<Self::Reflect>), Error>;
}

/// compact primary key pager
pub struct PkPager<S: PagerSource, R: PagerReflect<S::Entity>> {
    /// last primary
    last_id: i32,
    /// last direction(relative)
    last_direction: bool,
    /// original direction
    direction: bool,
    /// data
    data: <<PkPager<S, R> as Pager>::Source as PagerSource>::Data,
    source: PhantomData<S>,
    reflect: PhantomData<R>,
}

impl<S: PagerSource, R: PagerReflect<S::Entity>> Dump for PkPager<S, R> {
    fn serialize(self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(2);
        let mut type_number = S::TYPE_NUMBER & 0xfc;
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
        if ((notation ^ S::TYPE_NUMBER) & 0xfc) != 0 {
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
        let data = S::Data::deserialize(c.as_slice())?;

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
impl<S: PagerSource, R: PagerReflect<S::Entity>> Pager for PkPager<S, R> {
    type Source = S;
    type Reflect = R;

    async fn fetch(
        mut self,
        auth: &Auth,
        size: u64,
        offset: u64,
        rel_dir: bool,
    ) -> Result<(Self, Vec<Self::Reflect>), Error> {
        let paginator = PaginatePkBuilder::default()
            .pk(<S as PagerSource>::ID)
            .include(self.last_direction ^ rel_dir)
            .rev(self.direction ^ rel_dir)
            .last_pk(self.last_id)
            .build()
            .unwrap();

        self.last_direction = rel_dir;

        let query = paginator
            .apply(S::filter(auth, &self.data))
            .limit(size)
            .offset(offset);
        let models = R::all(query).await?;

        if let Some(model) = models.last() {
            self.last_id = R::get_id(model);
            return Ok((self, models));
        }

        Err(Error::NotInDB(S::Entity::DEBUG_NAME))
    }
    async fn new_fetch(
        data: S::Data,
        auth: &Auth,
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
        pager.fetch(auth, size, offset, abs_dir).await
    }
}

pub trait PagerSortSource<R>
where
    Self: PagerSource,
{
    /// get sort column
    fn sort_col(data: &Self::Data) -> impl ColumnTrait;
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone;
    /// save last value in column
    fn save_val(data: &mut Self::Data, model: &R) -> impl ColumnTrait;
}

/// compact column pager
pub struct ColPager<S: PagerSortSource<R>, R: PagerReflect<S::Entity>> {
    /// last primary
    last_id: i32,
    /// last direction(relative)
    last_direction: bool,
    /// original direction
    direction: bool,
    data: S::Data,
    source: PhantomData<S>,
    reflect: PhantomData<R>,
}

impl<S: PagerSortSource<R>, R: PagerReflect<S::Entity>> Dump for ColPager<S, R> {
    fn serialize(self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(2);
        let mut type_number = S::TYPE_NUMBER & 0xfc;
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
        if ((notation ^ S::TYPE_NUMBER) & 0xfc) != 0 {
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
        let data = S::Data::deserialize(c.as_slice())?;

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
impl<S: PagerSortSource<R>, R: PagerReflect<S::Entity>> Pager for ColPager<S, R> {
    type Source = S;
    type Reflect = R;

    async fn fetch(
        mut self,
        auth: &Auth,
        size: u64,
        offset: u64,
        rel_dir: bool,
    ) -> Result<(Self, Vec<Self::Reflect>), Error> {
        let col = S::sort_col(&self.data);
        let val = S::get_val(&self.data);

        let paginator = PaginateColBuilder::default()
            .pk(<S as PagerSource>::ID)
            .include(self.last_direction ^ rel_dir)
            .rev(self.direction ^ rel_dir)
            .col(col)
            .last_value(val)
            .last_pk(self.last_id)
            .build()
            .unwrap();

        self.last_direction = rel_dir;

        let query = paginator
            .apply(S::filter(auth, &self.data))
            .limit(size)
            .offset(offset);
        let models = R::all(query).await?;

        if let Some(model) = models.last() {
            self.last_id = R::get_id(model);
            return Ok((self, models));
        }

        Err(Error::NotInDB(S::Entity::DEBUG_NAME))
    }
    async fn new_fetch(
        data: S::Data,
        auth: &Auth,
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
        pager.fetch(auth, size, offset, abs_dir).await
    }
}

use sea_orm::{sea_query::SimpleExpr, *};

#[inline]
pub fn order_by_bool<E: EntityTrait>(
    query: Select<E>,
    col: impl ColumnTrait,
    rev: bool,
) -> Select<E> {
    let ord = match rev {
        true => Order::Desc,
        false => Order::Asc,
    };
    query.order_by(col, ord)
}
/// short-hand for gt,gte,lt,lte
///
/// true for desc
// included and asc=>gte
// excluded and asc=>gt
// included and desc=>lte
// excluded and desc=>lt
#[inline]
pub fn com_eq(eq: bool, ord: bool, val: impl Into<Value>, col: impl ColumnTrait) -> SimpleExpr {
    match eq {
        true => match ord {
            true => ColumnTrait::lte(&col, val),
            false => ColumnTrait::gte(&col, val),
        },
        false => match ord {
            true => ColumnTrait::lt(&col, val),
            false => ColumnTrait::gt(&col, val),
        },
    }
}

#[derive(derive_builder::Builder)]
#[builder(pattern = "owned")]
pub struct PaginateCol<PK: ColumnTrait, COL: ColumnTrait, CV: Into<Value>> {
    include: bool,
    rev: bool,
    pk: PK,
    col: COL,
    last_pk: i32,
    last_value: CV,
}

impl<PK: ColumnTrait, COL: ColumnTrait, CV: Into<Value> + Clone> PaginateCol<PK, COL, CV> {
    pub fn apply<E: EntityTrait>(self, query: Select<E>) -> Select<E> {
        let _ord = match self.rev {
            true => Order::Desc,
            false => Order::Asc,
        };
        // WHERE created >= $<after> and (id >= $<id> OR created > $<after>)
        let left = com_eq(true, self.rev, self.last_value.clone(), self.col);

        let right = {
            let left = com_eq(self.include, self.rev, self.last_pk, self.pk);
            let right = com_eq(false, self.rev, self.last_value, self.col);
            left.or(right)
        };

        let query = query.filter(left.and(right));

        let query = order_by_bool(query, self.pk, self.rev);

        order_by_bool(query, self.col, self.rev)
    }
}

#[derive(derive_builder::Builder)]
pub struct PaginatePk<PK: ColumnTrait> {
    include: bool,
    rev: bool,
    pk: PK,
    last_pk: i32,
}

impl<PK: ColumnTrait> PaginatePk<PK> {
    pub fn apply<E: EntityTrait>(self, query: Select<E>) -> Select<E> {
        let query = query.filter(com_eq(self.include, self.rev, self.last_pk, self.pk));
        let _ord = match self.rev {
            true => Order::Desc,
            false => Order::Asc,
        };

        order_by_bool(query, self.pk, self.rev)
    }
}
