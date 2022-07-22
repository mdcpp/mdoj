// use entity::question_table::*;
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220120_000001_create_post_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // manager
        //     .create_table(
        //         Table::create()
        //             .table(Entity)
        //             .if_not_exists()
        //             .to_owned(),
        //     )
        //     .await
        todo!()
    }

    // if you are against backward migrations, you do not have to impl this
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // manager
        //     .drop_table(Table::drop().table(Entity).to_owned())
        //     .await
        todo!()
    }
}
