use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "problem")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    #[sea_orm(nullable)]
    pub contest_id: Option<i32>,
    pub accept_count: i32,
    pub submit_count: u32,
    #[sea_orm(column_type = "Float")]
    pub ac_rate: f32,
    pub memory: i64,
    pub time: i64,
    pub difficulty: u32,
    pub public: bool,
    pub tags: String,
    pub title: String,
    pub content: String,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time", on_update = "current_timestamp")]
    pub update_at: chrono::NaiveDateTime,
    pub match_rule: i32,
    pub order: f32,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct PartialModel {
    pub id: i32,
    pub user_id: i32,
    pub contest_id: Option<i32>,
    pub submit_count: u32,
    pub ac_rate: f32,
    pub public: bool,
    pub title: String,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct IdModel {
    pub id: i32,
    pub user_id: i32,
    pub contest_id: Option<i32>,
    pub public: bool,
}

impl IdModel {
    /// create new model with only id(foreign and pirmary), useful for query
    ///
    /// Be careful never save it
    pub fn upgrade(self) -> Model {
        Model {
            id: self.id,
            user_id: self.user_id,
            contest_id: self.contest_id,
            accept_count: Default::default(),
            submit_count: Default::default(),
            ac_rate: Default::default(),
            memory: Default::default(),
            time: Default::default(),
            difficulty: Default::default(),
            public: self.public,
            tags: Default::default(),
            title: Default::default(),
            content: Default::default(),
            create_at: Default::default(),
            update_at: Default::default(),
            match_rule: Default::default(),
            order: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::education::Entity")]
    Education,
    #[sea_orm(has_many = "super::submit::Entity")]
    Submit,
    #[sea_orm(has_many = "super::chat::Entity")]
    Chat,
    #[sea_orm(has_many = "super::test::Entity")]
    Test,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::contest::Entity",
        from = "Column::ContestId",
        to = "super::contest::Column::Id"
    )]
    Contest,
}

impl Related<super::education::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Education.def()
    }
}

impl Related<super::submit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Submit.def()
    }
}

impl Related<super::chat::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chat.def()
    }
}

impl Related<super::test::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Test.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::contest::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Contest.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl super::DebugName for Entity {
    const DEBUG_NAME: &'static str = "problem";
}

impl super::ParentalTrait for Entity {
    const COL_ID: Column = Column::Id;

    fn related_filter(auth: &Auth) -> Select<Entity> {
        match user::Model::new_with_auth(auth) {
            Some(user) => user
                .find_linked(user::UserToProblem)
                .join_as(
                    JoinType::FullOuterJoin,
                    contest::Relation::Hoster.def().rev(),
                    Alias::new("own_problem"),
                )
                .join_as(
                    JoinType::FullOuterJoin,
                    user::Relation::PublicProblem.def(),
                    Alias::new("problem_unused"),
                )
                .distinct(),
            None => Entity::find().filter(Column::Public.eq(true)),
        }
    }
}

impl super::Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.can_root() {
                return Ok(query);
            }
            return Ok(query.filter(Column::Public.eq(true).or(Column::UserId.eq(user_id))));
        }
        Ok(query.filter(Column::Public.eq(true)))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() {
            return Ok(query);
        }
        if perm.can_manage_problem() {
            return Ok(query.filter(Column::UserId.eq(user_id)));
        }
        Err(Error::PermissionDeny("Can't write problem"))
    }
}
