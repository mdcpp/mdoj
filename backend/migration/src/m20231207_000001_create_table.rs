use paste::paste;
use sea_orm::{DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;

// static UPDATE_AT: &str = "DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP";
static UPDATE_AT: &str = "DEFAULT CURRENT_TIMESTAMP";
static CREATE_AT: &str = "DEFAULT CURRENT_TIMESTAMP";

macro_rules! index {
    ($manager:expr,$table:ident,$col:ident) => {
        paste! {
            $manager
            .create_index(
                Index::create()
                    .name(
                        concat!(
                            "idx-",
                            stringify!($table),
                            "-",
                            stringify!($col),
                        ).to_lowercase()
                    )
                    .table($table::Table)
                    .col($table::$col)
                    .to_owned(),
            )
            .await?;
        }
    };
}

#[derive(Iden)]
enum Announcement {
    Table,
    Id,
    ContestId,
    UserId,
    Public,
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
pub enum Problem {
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
    Order,
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
enum Testcase {
    Table,
    Id,
    UserId,
    ProblemId,
    Input,
    Output,
    Score,
    Order,
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
enum Chat {
    Table,
    Id,
    UserId,
    ProblemId,
    CreateAt,
    Message,
}

#[derive(Iden)]
enum UserContest {
    Table,
    UserId,
    ContestId,
    Score,
    Id,
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
                    .col(
                        ColumnDef::new(Announcement::Public)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Announcement::Content)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Announcement::ContestId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-announcement-contest")
                            .from(Announcement::Table, Announcement::ContestId)
                            .to(Contest::Table, Contest::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Announcement::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-announcement-user")
                            .from(Announcement::Table, Announcement::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .col(
                        ColumnDef::new(Announcement::CreateAt)
                            .date_time()
                            .not_null()
                            .extra(CREATE_AT.to_string()),
                    )
                    .col(
                        ColumnDef::new(Announcement::UpdateAt)
                            .date_time()
                            .not_null()
                            .extra(UPDATE_AT.to_string()),
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
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-announcement-user-hoster")
                            .from(Contest::Table, Contest::Hoster)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .col(ColumnDef::new(Contest::Begin).date_time().not_null())
                    .col(ColumnDef::new(Contest::End).date_time().not_null())
                    .col(ColumnDef::new(Contest::Title).text().not_null())
                    .col(
                        ColumnDef::new(Contest::Content)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Contest::Tags).text().not_null().default(""))
                    .col(ColumnDef::new(Contest::Password).binary().null())
                    .col(
                        ColumnDef::new(Contest::CreateAt)
                            .date_time()
                            .not_null()
                            .extra(CREATE_AT.to_string()),
                    )
                    .col(
                        ColumnDef::new(Contest::UpdateAt)
                            .date_time()
                            .not_null()
                            .extra(UPDATE_AT.to_string()),
                    )
                    .col(
                        ColumnDef::new(Contest::Public)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
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
                            .to(Problem::Table, Problem::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .col(ColumnDef::new(Education::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-education-user")
                            .from(Education::Table, Education::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .col(
                        ColumnDef::new(Education::Tags)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Education::Title).text().not_null())
                    .col(
                        ColumnDef::new(Education::Content)
                            .text()
                            .not_null()
                            .default(""),
                    )
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
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Problem::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-problem-user")
                            .from(Problem::Table, Problem::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .col(ColumnDef::new(Problem::ContestId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-problem-contest")
                            .from(Problem::Table, Problem::ContestId)
                            .to(Contest::Table, Contest::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .col(
                        ColumnDef::new(Problem::AcceptCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Problem::SubmitCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Problem::AcRate)
                            .float()
                            .not_null()
                            .default(0.0),
                    )
                    .col(ColumnDef::new(Problem::Memory).big_integer().not_null())
                    .col(ColumnDef::new(Problem::Time).big_integer().not_null())
                    .col(
                        ColumnDef::new(Problem::Difficulty)
                            .unsigned()
                            .not_null()
                            .default(512),
                    )
                    .col(
                        ColumnDef::new(Problem::Public)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Problem::Tags).text().not_null().default(""))
                    .col(ColumnDef::new(Problem::Title).text().not_null())
                    .col(
                        ColumnDef::new(Problem::Content)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(Problem::CreateAt)
                            .date_time()
                            .not_null()
                            .extra(CREATE_AT.to_string()),
                    )
                    .col(
                        ColumnDef::new(Problem::UpdateAt)
                            .date_time()
                            .not_null()
                            .extra(UPDATE_AT.to_string()),
                    )
                    .col(ColumnDef::new(Problem::MatchRule).integer().not_null())
                    .col(ColumnDef::new(Problem::Order).float().not_null())
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
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Submit::UserId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-submit-user")
                            .from(Submit::Table, Submit::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Submit::ProblemId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-submit-problem")
                            .from(Submit::Table, Submit::ProblemId)
                            .to(Problem::Table, Problem::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(Submit::UploadAt)
                            .not_null()
                            .extra(CREATE_AT.to_string()),
                    )
                    .col(ColumnDef::new(Submit::Time).big_integer().null())
                    .col(ColumnDef::new(Submit::Accuracy).big_integer().null())
                    .col(
                        ColumnDef::new(Submit::Committed)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Submit::Lang).text().not_null())
                    .col(ColumnDef::new(Submit::Code).not_null().binary())
                    .col(ColumnDef::new(Submit::Memory).big_integer().null())
                    .col(
                        ColumnDef::new(Submit::PassCase)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Submit::Status).unsigned().null())
                    .col(
                        ColumnDef::new(Submit::Accept)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Submit::Score)
                            .unsigned()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Testcase::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Testcase::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Testcase::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-test-user")
                            .from(Testcase::Table, Testcase::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Testcase::ProblemId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-test-user")
                            .from(Testcase::Table, Testcase::ProblemId)
                            .to(Problem::Table, Problem::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Testcase::Input).binary().not_null())
                    .col(ColumnDef::new(Testcase::Output).binary().not_null())
                    .col(
                        ColumnDef::new(Testcase::Score)
                            .unsigned()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Testcase::Order)
                            .float()
                            .not_null()
                            .default(0.0),
                    )
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
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Token::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-token-user")
                            .from(Token::Table, Token::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Token::Rand).binary().not_null())
                    .col(
                        ColumnDef::new(Token::Permission)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Token::Expiry).date_time().not_null())
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
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(User::Permission)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(User::Score)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(User::Username)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(User::Password).binary().not_null())
                    .col(
                        ColumnDef::new(User::CreateAt)
                            .date_time()
                            .not_null()
                            .extra(CREATE_AT.to_string()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(UserContest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserContest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserContest::ContestId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-pivot-contest-user")
                            .from(UserContest::Table, UserContest::ContestId)
                            .to(Contest::Table, Contest::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(UserContest::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-pivot-user-contest")
                            .from(UserContest::Table, UserContest::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(UserContest::Score)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Chat::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Chat::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Chat::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-chat-user")
                            .from(Chat::Table, Chat::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Chat::ProblemId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-chat-problem")
                            .from(Chat::Table, Chat::ProblemId)
                            .to(Problem::Table, Problem::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(Chat::CreateAt)
                            .date_time()
                            .not_null()
                            .extra(CREATE_AT.to_string()),
                    )
                    .col(ColumnDef::new(Chat::Message).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-problem-text")
                    .table(Problem::Table)
                    .col(Problem::Tags)
                    .col(Problem::Title)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx-education-text")
                    .table(Education::Table)
                    .col(Education::Tags)
                    .col(Education::Title)
                    .to_owned(),
            )
            .await?;

        index!(manager, Problem, Public);
        index!(manager, Problem, SubmitCount);
        index!(manager, Problem, AcRate);
        index!(manager, Problem, AcceptCount);
        index!(manager, Problem, Difficulty);
        index!(manager, Problem, Order);
        index!(manager, Problem, Content);
        index!(manager, Problem, Title);

        index!(manager, Submit, Committed);
        index!(manager, Submit, Time);
        index!(manager, Submit, Memory);
        index!(manager, Submit, UserId);

        index!(manager, Contest, Hoster);
        index!(manager, Contest, Public);
        index!(manager, Contest, End);
        index!(manager, Contest, Begin);

        index!(manager, User, Score);
        index!(manager, User, Username);

        index!(manager, Token, Rand);
        index!(manager, Token, Expiry);

        index!(manager, Chat, CreateAt);

        manager
            .get_connection()
            .execute(
                Statement::from_string(DatabaseBackend::Sqlite, "PRAGMA journal_mode = WAL")
                    .to_owned(),
            )
            .await?;
        manager
            .get_connection()
            .execute(
                Statement::from_string(DatabaseBackend::Sqlite, "PRAGMA synchronous = NORMAL")
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
        manager
            .drop_table(Table::drop().table(Chat::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Contest::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Education::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Problem::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Submit::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Testcase::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Token::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(UserContest::Table).to_owned())
            .await?;
        Ok(())
    }
}
