use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "submit")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: Option<i32>,
    pub problem_id: i32,
    #[sea_orm(column_type = "Time")]
    pub upload_at: chrono::NaiveDateTime,
    #[sea_orm(nullable)]
    pub time: Option<i64>,
    #[sea_orm(nullable)]
    pub accuracy: Option<i64>,
    pub committed: bool,
    pub lang: String,
    #[sea_orm(column_type = "Binary(BlobSize::Blob(None))")]
    pub code: Vec<u8>,
    #[sea_orm(nullable)]
    pub memory: Option<i64>,
    pub pass_case: i32,
    #[sea_orm(nullable)]
    pub status: Option<u32>,
    pub accept: bool,
    pub score: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::problem::Entity",
        from = "Column::ProblemId",
        to = "super::problem::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Problem,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    User,
}

impl Related<super::problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl super::DebugName for Entity {
    const DEBUG_NAME: &'static str = "submit";
}

impl super::Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, _: &Auth) -> Result<S, Error> {
        Ok(query)
    }

    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_manage_submit() || perm.can_root() {
                return Ok(query);
            }
        }
        Err(Error::Unauthenticated)
    }
}