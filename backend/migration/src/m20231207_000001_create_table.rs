use std::ptr::null;

use sea_orm_migration::{prelude::*, seaql_migrations::PrimaryKey};

#[derive(Iden)]
enum Announcement {
    Table,
    Id,
    Title,
    Content,
    CreateAt,
    UpdateAt,
}
#[derive(Iden)]
enum Contest {
    Table,
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
#[derive(Iden)]
enum Education {
    Table,
    Id,
    ProblemId,
    UserId,
    Tags,
    Title,
    Content,
}
#[derive(Iden)]
enum Problem {
    Table,
    Id,
    UserId,
    ContestId,
    AcceptCount,
    SubmitCount,
    AcRate,
    Memory,
    Time,
    Difficulty,
    Public,
    Tags,
    Title,
    Content,
    CreateAt,
    UpdateAt,
    MatchRule,
}

#[derive(Iden)]
enum Submit {
    Table,
    Id,
    UserId,
    ProblemId,
    UploadAt,
    Time,
    Accuracy,
    Committed,
    Lang,
    Code,
    Memory,
    PassCase,
    Status,
    Accept,
    Score,
}
#[derive(Iden)]
enum Test {
    Table,
    Id,
    UserId,
    ProblemId,
    Input,
    Output,
    Score,
}
#[derive(Iden)]
enum Token {
    Table,
    Id,
    UserId,
    Rand,
    Permission,
    Expiry,
}
#[derive(Iden)]
enum User {
    Table,
    Id,
    Permission,
    Score,
    Username,
    Password,
    CreateAt,
}
#[derive(Iden)]
enum UserContest {
    Table,
    Id,
    UserId,
    ContestId,
    Score,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Announcement::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Announcement::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Announcement::Title).string().not_null())
                    .col(ColumnDef::new(Announcement::Content).string().default(""))
                    .col(
                        ColumnDef::new(Announcement::CreateAt)
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Announcement::UpdateAt).not_null().extra(
                            "DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string(),
                        ),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Contest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Contest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Contest::Hoster).integer().not_null())
                    .col(ColumnDef::new(Contest::Begin).time().not_null())
                    .col(ColumnDef::new(Contest::End).time().not_null())
                    .col(ColumnDef::new(Contest::Title).text().not_null())
                    .col(ColumnDef::new(Contest::Content).text().default(""))
                    .col(ColumnDef::new(Contest::Tags).text().default(""))
                    .col(ColumnDef::new(Contest::Password).binary().null())
                    .col(
                        ColumnDef::new(Contest::CreateAt)
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Contest::UpdateAt).not_null().extra(
                            "DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string(),
                        ),
                    )
                    .col(ColumnDef::new(Contest::Public).boolean().default(false))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Education::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Education::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Education::ProblemId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-education-problem")
                            .from(Education::Table, Education::ProblemId)
                            .to(Problem::Table, Problem::Id),
                    )
                    .col(ColumnDef::new(Education::UserId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-education-user")
                            .from(Education::Table, Education::UserId)
                            .to(User::Table, User::Id),
                    )
                    .col(ColumnDef::new(Education::Tags).text().default(""))
                    .col(ColumnDef::new(Education::Title).text().not_null())
                    .col(ColumnDef::new(Education::Content).text().default(""))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Problem::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Problem::Id)
                            .integer()
                            .not_null()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(Problem::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-problem-user")
                            .from(Problem::Table, Problem::UserId)
                            .to(User::Table, User::Id),
                    )
                    .col(ColumnDef::new(Problem::ContestId).integer().null())
                    .col(ColumnDef::new(Problem::AcceptCount).integer().default(0))
                    .col(ColumnDef::new(Problem::SubmitCount).integer().default(0))
                    .col(ColumnDef::new(Problem::AcRate).float().default(0.0))
                    .col(ColumnDef::new(Problem::Memory).big_unsigned().not_null())
                    .col(ColumnDef::new(Problem::Time).big_unsigned().not_null())
                    .col(ColumnDef::new(Problem::Difficulty).unsigned().default(512))
                    .col(ColumnDef::new(Problem::Public).boolean().default(false))
                    .col(ColumnDef::new(Problem::Tags).text().default(""))
                    .col(ColumnDef::new(Problem::Title).text().not_null())
                    .col(ColumnDef::new(Problem::Content).text().default(""))
                    .col(
                        ColumnDef::new(Problem::CreateAt)
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Problem::UpdateAt).not_null().extra(
                            "DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string(),
                        ),
                    )
                    .col(ColumnDef::new(Problem::MatchRule).integer().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Submit::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Submit::Id)
                            .integer()
                            .primary_key()
                            .auto_increment()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Submit::UserId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-submit-user")
                            .from(Submit::Table, Submit::UserId)
                            .to(User::Table, User::Id),
                    )
                    .col(ColumnDef::new(Submit::ProblemId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-submit-problem")
                            .from(Submit::Table, Submit::ProblemId)
                            .to(Problem::Table, Problem::Id),
                    )
                    .col(
                        ColumnDef::new(Submit::UploadAt)
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(ColumnDef::new(Submit::Time).big_unsigned().null())
                    .col(ColumnDef::new(Submit::Accuracy).big_unsigned().null())
                    .col(ColumnDef::new(Submit::Committed).default(false))
                    .col(ColumnDef::new(Submit::Lang).text().not_null())
                    .col(ColumnDef::new(Submit::Code).binary())
                    .col(ColumnDef::new(Submit::Memory).big_unsigned().null())
                    .col(ColumnDef::new(Submit::PassCase).integer().default(0))
                    .col(ColumnDef::new(Submit::Status).unsigned().null())
                    .col(ColumnDef::new(Submit::Accept).boolean().default(false))
                    .col(ColumnDef::new(Submit::Score).unsigned().default(0))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Test::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Test::Id)
                            .primary_key()
                            .not_null()
                            .auto_increment()
                            .integer(),
                    )
                    .col(ColumnDef::new(Test::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-test-user")
                            .from(Test::Table, Test::UserId)
                            .to(User::Table, User::Id),
                    )
                    .col(ColumnDef::new(Test::ProblemId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-test-user")
                            .from(Test::Table, Test::ProblemId)
                            .to(Problem::Table, Problem::Id),
                    )
                    .col(ColumnDef::new(Test::Input).binary().not_null())
                    .col(ColumnDef::new(Test::Output).binary().not_null())
                    .col(ColumnDef::new(Test::Score).unsigned().default(0))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Token::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Token::Id)
                            .primary_key()
                            .not_null()
                            .auto_increment()
                            .integer(),
                    )
                    .col(ColumnDef::new(Token::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-token-user")
                            .from(Token::Table, Token::UserId)
                            .to(User::Table, User::Id),
                    )
                    .col(ColumnDef::new(Token::Rand).binary().not_null())
                    .col(ColumnDef::new(Token::Permission).big_unsigned().default(0))
                    .col(ColumnDef::new(Token::Expiry).time().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(User::Id)
                            .not_null()
                            .primary_key()
                            .auto_increment()
                            .integer(),
                    )
                    .col(ColumnDef::new(User::Permission).big_unsigned().default(0))
                    .col(ColumnDef::new(User::Score).unsigned().default(0))
                    .col(ColumnDef::new(User::Username).text().not_null())
                    .col(ColumnDef::new(User::Password).binary().not_null())
                    .col(
                        ColumnDef::new(User::CreateAt)
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(UserContest::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserContest::ContestId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-pivot-contest-user")
                            .from(UserContest::Table, UserContest::ContestId)
                            .to(Contest::Table, Contest::Id),
                    )
                    .col(ColumnDef::new(UserContest::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-pivot-user-contest")
                            .from(UserContest::Table, UserContest::UserId)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(Announcement::Table).to_owned())
            .await?;
        todo!()
    }
}
