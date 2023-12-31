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
pub fn com_eq(
    eq: bool,
    ord: bool,
    val: impl Into<Value>,
    col: impl ColumnTrait,
) -> SimpleExpr {
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
        let _ord = match self.rev {
            true => Order::Desc,
            false => Order::Asc,
        };
        // WHERE created >= $<after> and (id >= $<id> OR created > $<after>)
        let left = com_eq(true, self.rev, self.last_value, self.col);

        let right = {
            let left = com_eq(self.include, self.rev, self.last_id, self.pk);
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
    last: i32,
}

impl<PK: ColumnTrait> PaginatePk<PK> {
    pub fn apply<E: EntityTrait>(self, query: Select<E>) -> Select<E> {
        let query = query.filter(com_eq(self.include, self.rev, self.last, self.pk));
        let _ord = match self.rev {
            true => Order::Desc,
            false => Order::Asc,
        };

        order_by_bool(query, self.pk, self.rev)
    }
}
