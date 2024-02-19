//! a collection of helper function

use std::ops::Deref;

use sea_orm::*;
use sea_query::{
    types, Alias, Expr, Order, Query, SelectStatement, SimpleExpr, SubQueryStatement, Value,
};

use crate::util::error::Error;

use super::paginator::Paginate;

/// bool to order
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

/// Builder to paginate by column
///
/// It's call `Paginate` instead of `Paginator` because it's stateless
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

impl<PK, COL, CV, E> Paginate<E> for PaginateCol<PK, COL, CV>
where
    PK: ColumnTrait,
    COL: ColumnTrait,
    CV: Into<Value> + Clone,
    E: EntityTrait,
{
    /// Apply pagination effect on a Select(sea_orm)
    ///
    /// be careful not to run order_by before applying pagination
    fn apply(self, query: Select<E>) -> Select<E> {
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

/// Builder to paginate by primary key
///
/// It's call `Paginate` instead of `Paginator` because it's stateless
#[derive(derive_builder::Builder)]
pub struct PaginatePk<PK: ColumnTrait> {
    include: bool,
    rev: bool,
    pk: PK,
    last_pk: i32,
}

impl<PK: ColumnTrait, E: EntityTrait> Paginate<E> for PaginatePk<PK> {
    fn apply(self, query: Select<E>) -> Select<E> {
        let query = query.filter(com_eq(self.include, self.rev, self.last_pk, self.pk));

        order_by_bool(query, self.pk, self.rev)
    }
}

/// Builder for counting how many elements could be if we apply
/// no `LIMIT` and 0 `OFFSET` to the query, with maximum result be `max`
///
/// It's fast and cost `O(max(n,max))` to compute, but inaccurate if there is more than `max`
///
/// It actually build sql query like this
///
/// ```sql
/// SELECT CASE
///   WHEN EXISTS (SELECT $query LIMIT 1 OFFSET $max)
///   THEN (SELECT COUNT(*) $query) ELSE $max
/// END AS num_items;
/// ```
#[derive(derive_builder::Builder)]
pub struct MaxCount<E: EntityTrait> {
    query: Select<E>,
    max: u64,
}

impl<E: EntityTrait> MaxCount<E> {
    fn count_query(self) -> SelectStatement {
        let query_up = self.query.clone().limit(1).offset(self.max).into_query();
        let mut query_low = self.query.into_query();

        let query_up = Expr::exists(query_up);
        query_low.expr(Expr::col(types::Asterisk).count());

        let query_low = SimpleExpr::SubQuery(
            None,
            Box::new(SubQueryStatement::SelectStatement(query_low)),
        );

        let query_case = Expr::case(query_up, query_low).finally(self.max);

        Query::select()
            .expr_as(query_case, Alias::new("num_items"))
            .to_owned()
    }
    pub async fn count(self, db: &DatabaseConnection) -> Result<u64, Error> {
        let (query, param) = {
            let builder = db.get_database_backend().get_query_builder();
            self.count_query().build_any(builder.deref())
        };

        let stmt = Statement::from_sql_and_values(db.get_database_backend(), query, param);

        Ok(match db.query_one(stmt).await? {
            Some(res) => res.try_get::<i32>("", "num_items")? as u64,
            None => 0,
        })
    }
}
