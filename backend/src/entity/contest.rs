use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "contest")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub hoster: i32,
    #[sea_orm(column_type = "Time")]
    pub begin: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time")]
    pub end: chrono::NaiveDateTime,
    pub title: String,
    pub content: String,
    pub tags: String,
    #[sea_orm(column_type = "Binary(BlobSize::Blob(None))", nullable)]
    pub password: Option<Vec<u8>>,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time", on_update = "current_timestamp")]
    pub update_at: chrono::NaiveDateTime,
    pub public: bool,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct PartialContest {
    pub id: i32,
    pub hoster: i32,
    pub begin: chrono::NaiveDateTime,
    pub end: chrono::NaiveDateTime,
    pub title: String,
    pub password: Option<Vec<u8>>,
    pub public: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::announcement::Entity")]
    Announcement,
    #[sea_orm(has_many = "super::problem::Entity")]
    Problem,
    #[sea_orm(has_many = "super::user_contest::Entity")]
    UserContest,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::Hoster",
        to = "super::user::Column::Id"
    )]
    Hoster,
}

impl Related<super::announcement::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Announcement.def()
    }
}

impl Related<super::user_contest::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserContest.def()
    }
}

impl Related<super::problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_contest::Relation::User.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::user_contest::Relation::Contest.def().rev())
    }
}

// impl Related<Entity> for Entity {
//     fn to() -> RelationDef {
//         Relation::Public.def()
//     }
// }

impl ActiveModelBehavior for ActiveModel {}

impl super::DebugName for Entity {
    const DEBUG_NAME: &'static str = "contest";
}

#[tonic::async_trait]
impl ParentalTrait for Entity {
    const COL_ID: Column = Column::Id;

    async fn related_filter(auth: &Auth) -> Result<Select<Entity>, Error> {
        let db = DB.get().unwrap();
        Ok(match auth.get_user(db).await {
            Ok(user) => user
                .find_related(Entity)
                .join_as(
                    JoinType::FullOuterJoin,
                    Relation::Hoster.def().rev(),
                    Alias::new("own_contest"),
                )
                .join_as(
                    JoinType::FullOuterJoin,
                    user::Relation::PublicContest.def(),
                    Alias::new("user_contest_unused"),
                ),
            Err(_) => Entity::find().filter(Column::Public.eq(true)),
        })
    }
}

impl super::Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.can_root() {
                return Ok(query);
            }
            return Ok(query.filter(Column::Public.eq(true).or(Column::Hoster.eq(user_id))));
        }
        Ok(query.filter(Column::Public.eq(true)))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() {
            return Ok(query);
        }
        if perm.can_manage_contest() {
            return Ok(query.filter(Column::Hoster.eq(user_id)));
        }
        Err(Error::PermissionDeny("Can't write contest"))
    }
}
