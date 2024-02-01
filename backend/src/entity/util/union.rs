// use sea_orm::{EntityTrait, FromQueryResult, QuerySelect, QueryTrait, Select, ColumnTrait};
// use sea_query::{SelectStatement, UnionType};

// pub trait IntoSelectColumn<E> where E:EntityTrait,Self:FromQueryResult, E::Column: ColumnTrait{
//     fn columns() -> E::Column;
//     // where
//         // I: IntoIterator<Item = E::Column>;
//     // const Columns: &'static [E::Column];
// }

// pub trait UnionTrait<P,E>
// where
//     E: EntityTrait,
//     P: IntoSelectColumn<E>,
// {
//     fn unions<I>(self, others: I) -> SelectStatement
//     where
//         I: IntoIterator<Item = Self>;
// }

// impl<P, E> UnionTrait<P,E> for Select<E>
// where
//     E: EntityTrait,
//     P: IntoSelectColumn<E>,
// {
//     fn unions<I>(self, others: I) -> SelectStatement
//     where
//         I: IntoIterator<Item = Self>,
//     {
//         let mut statment = self
//             .select_only()
//             .column(P::columns())
//             .into_query()
//             .to_owned();
//         for other in others {
//             let other = other.into_query().to_owned();
//             statment = statment.union(UnionType::Distinct, other).to_owned();
//         }
//         todo!()
//     }
// }
