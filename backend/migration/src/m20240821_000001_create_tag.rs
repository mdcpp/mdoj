use crate::m20231207_000001_create_table::Problem;
use sea_orm::{DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(Iden)]
enum Tag {
    Table,
    Id,
    Name,
}

#[derive(Iden)]
enum TagProblem {
    Table,
    Id,
    ProblemId,
    TagId,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tag::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tag::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Tag::Name).string().unique_key().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx-tag-name".to_lowercase())
                    .table(Tag::Table)
                    .col(Tag::Name)
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(TagProblem::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TagProblem::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TagProblem::ProblemId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-pivot-problem-tag")
                            .from(TagProblem::Table, TagProblem::ProblemId)
                            .to(Problem::Table, Problem::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(TagProblem::TagId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-pivot-tag-problem")
                            .from(TagProblem::Table, TagProblem::TagId)
                            .to(Tag::Table, Tag::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }
}
