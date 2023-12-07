use sea_orm_migration::prelude::*;

#[derive(Iden)]
enum Announcement{
    Id,
    Title,
    Content,
    CreateAt,
    UpdateAt,
}
enum Contest {
    Id,
    Hoster,
    Begin,
    End,
    Title,
    Content,
    Tags,
    Password,
    CreateAt,
    UpdateAt,
    Public,
}
enum Education{
        Id,
        ProblemId,
        UserId,
        Tags,
        Title,
        Content,
}
#[derive(Iden)]
enum Post {
    Table,
    Id,
    Title,
    Text,
}
#[derive(Iden)]
enum Problem
    {
        id,
        user_id,
        contest_id,
        accept_count,
        submit_count,
        ac_rate,
        memory,
        time,
        difficulty,
        public,
        tags,
        title,
        content,
        create_at,
        UpdateAt,
        MatchRule,
    }

enum Submit{
    id,
    user_id,
    problem_id,
    upload_at,
    time,
    accuracy,
    committed,
    lang,
    code,
    memory,
    pass_case,
    status,
    accept,
    score,
}
enum Test{
    id,
    user_id,
    problem_id,
    input,
    output,
    score,
}
enum Token{
    id,
    user_id,
    rand,
    permission,
    expiry,
}
enum User {
    id,
    permission,
    score,
    username,
    password,
    create_at,
}
enum UserContest{
    id,
    user_id,
    contest_id,
    score,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();

        manager
            .create_table(
                Table::create()
                    .table(Post::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Post::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Post::Title).string().not_null())
                    .col(ColumnDef::new(Post::Text).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();

        manager
            .drop_table(Table::drop().table(Post::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
