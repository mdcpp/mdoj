use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .col(
                        ColumnDef::new(User::Id)
                            .integer()
                            .auto_increment()
                            .primary_key().not_null(),
                    )
                    .col(ColumnDef::new(User::Name).char().not_null())
                    .col(ColumnDef::new(User::HashPwd).binary().not_null())
                    .col(ColumnDef::new(User::Permission).big_integer().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Token::Table)
                    .col(
                        ColumnDef::new(Token::Id)
                            .integer()
                            .auto_increment()
                            .primary_key().not_null(),
                    )
                    .col(ColumnDef::new(Token::Content).big_integer().not_null())
                    .col(ColumnDef::new(Token::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-user_id")
                            .from(Token::Table, Token::UserId)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Group::Table)
                    .col(
                        ColumnDef::new(Group::Id)
                            .integer()
                            .auto_increment()
                            .primary_key().not_null(),
                    )
                    .col(ColumnDef::new(Group::Name).char())
                    .col(ColumnDef::new(Group::OwnerId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-user")
                            .from(Group::Table, Group::OwnerId)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(UserGroup::Table)
                    .col(
                        ColumnDef::new(UserGroup::Id)
                            .integer()
                            .auto_increment()
                            .primary_key().not_null(),
                    )
                    .col(
                        ColumnDef::new(UserGroup::Permission)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(UserGroup::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-user")
                            .from(UserGroup::Table, UserGroup::UserId)
                            .to(User::Table, User::Id),
                    )
                    .col(ColumnDef::new(UserGroup::GroupId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-group")
                            .from(UserGroup::Table, UserGroup::GroupId)
                            .to(Group::Table, Group::Id),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserGroup::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Group::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Token::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum User {
    Table,
    Id,
    Name,
    HashPwd,
    Permission,
}

#[derive(Iden)]
enum Token {
    Table,
    Id,
    Content,
    UserId,
}

#[derive(Iden)]
enum Group {
    Table,
    Id,
    OwnerId,
    Name,
}

#[derive(Iden)]
enum UserGroup {
    Table,
    Id,
    UserId,
    GroupId,
    Permission,
}
