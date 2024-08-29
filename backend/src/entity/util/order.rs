use crate::util::with::WithDB;
use crate::util::with::WithDBTrait;
use sea_orm::*;
use tonic::async_trait;

#[async_trait]
pub trait ReOrder {
    async fn insert_last(self) -> Result<f32, DbErr>;
    async fn insert_after(self, pivot: i32) -> Result<f32, DbErr>;
    async fn insert_front(self) -> Result<f32, DbErr>;
}

#[derive(Default, EnumIter, DeriveColumn, Clone, Copy, Debug)]
enum RetValue {
    #[default]
    RetValue,
}
pub mod testcase {
    use super::*;
    use crate::entity::problem;
    use crate::entity::testcase::{Column, Entity};

    impl WithDBTrait for problem::IdModel {}

    #[async_trait]
    impl ReOrder for WithDB<'_, problem::IdModel> {
        async fn insert_last(self) -> Result<f32, DbErr> {
            Entity::find()
                .filter(Column::ProblemId.eq(self.1.id))
                .select_only()
                .column_as(Column::Order.max(), RetValue::default())
                .into_tuple()
                .one(self.0)
                .await
                .map(|x: Option<Option<f32>>| x.flatten().unwrap_or_default() + 1.0)
        }
        async fn insert_after(self, pivot: i32) -> Result<f32, DbErr> {
            let vals: Vec<f32> = Entity::find()
                .filter(Column::ProblemId.eq(self.1.id))
                .filter(Column::Order.gte(pivot))
                .select_only()
                .column_as(Column::Order.min(), RetValue::default())
                .limit(2)
                .into_tuple()
                .all(self.0)
                .await?;
            Ok(match vals.len() {
                1 => vals[0] + 1.0,
                2 => (vals[0] + vals[1]) * 0.5,
                _ => 0.0,
            })
        }
        async fn insert_front(self) -> Result<f32, DbErr> {
            Entity::find()
                .filter(Column::ProblemId.eq(self.1.id))
                .select_only()
                .column_as(Column::Order.min(), RetValue::default())
                .into_tuple()
                .one(self.0)
                .await
                .map(|x: Option<Option<f32>>| x.flatten().unwrap_or_default() - 1.0)
        }
    }
}

pub mod contest {
    use super::*;
    use crate::entity::contest;
    use crate::entity::problem::{Column, Entity};

    impl WithDBTrait for contest::IdModel {}
    #[async_trait]
    impl ReOrder for WithDB<'_, contest::IdModel> {
        async fn insert_last(self) -> Result<f32, DbErr> {
            Entity::find()
                .filter(Column::ContestId.eq(self.1.id))
                .select_only()
                .column_as(Column::Order.max(), RetValue::default())
                .into_tuple()
                .one(self.0)
                .await
                .map(|x: Option<Option<f32>>| x.flatten().unwrap_or_default() + 1.0)
        }
        async fn insert_after(self, pivot: i32) -> Result<f32, DbErr> {
            let vals: Vec<f32> = Entity::find()
                .filter(Column::ContestId.eq(self.1.id))
                .filter(Column::Order.gte(pivot))
                .select_only()
                .column_as(Column::Order.min(), RetValue::default())
                .limit(2)
                .into_tuple()
                .all(self.0)
                .await?;
            Ok(match vals.len() {
                1 => vals[0] + 1.0,
                2 => (vals[0] + vals[1]) * 0.5,
                _ => 0.0,
            })
        }
        async fn insert_front(self) -> Result<f32, DbErr> {
            Entity::find()
                .filter(Column::ContestId.eq(self.1.id))
                .select_only()
                .column_as(Column::Order.min(), RetValue::default())
                .into_tuple()
                .one(self.0)
                .await
                .map(|x: Option<Option<f32>>| x.flatten().unwrap_or_default() - 1.0)
        }
    }
}
