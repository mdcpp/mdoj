use std::marker::PhantomData;

use ::entity::*;
use sea_orm::{sea_query::SimpleExpr, PrimaryKeyToColumn, *};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{grpc::backend::SortBy, init::db::DB, server::Server};

use super::{HasParent, NoParent, PagerTrait, ParentalTrait};
use crate::endpoint::util::{auth::Auth, error::Error, filter::Filter};

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
pub fn compare_include(
    include: bool,
    order: bool,
    value: impl Into<Value>,
    col: impl ColumnTrait,
) -> SimpleExpr {
    match include {
        true => match order {
            true => ColumnTrait::lte(&col, value),
            false => ColumnTrait::gte(&col, value),
        },
        false => match order {
            true => ColumnTrait::lt(&col, value),
            false => ColumnTrait::gt(&col, value),
        },
    }
}

#[derive(derive_builder::Builder)]
pub struct PaginateCol<'a, PK: ColumnTrait, COL: ColumnTrait> {
    include: bool,
    rev: bool,
    pk: PK,
    col: COL,
    last_id: i32,
    last_value: &'a str,
}

impl<'a, PK: ColumnTrait, COL: ColumnTrait> PaginateCol<'a, PK, COL> {
    pub fn apply<E: EntityTrait>(self, query: Select<E>) -> Select<E> {
        let ord = match self.rev {
            true => Order::Desc,
            false => Order::Asc,
        };
        // WHERE created >= $<after> and (id >= $<id> OR created > $<after>)
        let left = compare_include(true, self.rev, self.last_value, self.col);

        let right = {
            let left = compare_include(self.include, self.rev, self.last_id, self.pk);
            let right = compare_include(false, self.rev, self.last_value, self.col);
            left.or(right)
        };

        let query = query.filter(left.and(right));

        let query = order_by_bool(query, self.pk, self.rev);
        let query = order_by_bool(query, self.col, self.rev);

        query
    }
}

#[derive(derive_builder::Builder)]
pub struct PaginatePk<PK: ColumnTrait> {
    include: bool,
    rev: bool,
    pk: PK,
    last: i32,
}

impl<PK: ColumnTrait> PaginatePk<PK> {
    pub fn apply<E: EntityTrait>(self, query: Select<E>) -> Select<E> {
        let query = query.filter(compare_include(self.include, self.rev, self.last, self.pk));
        let ord = match self.rev {
            true => Order::Desc,
            false => Order::Asc,
        };
        let query = order_by_bool(query, self.pk, self.rev);
        query
    }
}
