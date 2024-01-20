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

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct PartialModel {
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
        if auth.user_perm().admin(){
            return Ok(query);
        }
        Err(Error::Unauthenticated)
    }
}

#[async_trait]
impl PagerReflect<Entity> for PartialModel {
    fn get_id(&self) -> i32 {
        self.id
    }

    async fn all(query: Select<Entity>) -> Result<Vec<Self>, Error> {
        let db = DB.get().unwrap();
        query
            .into_model::<Self>()
            .all(db)
            .await
            .map_err(Into::<Error>::into)
    }
}

pub struct ParentPagerTrait;

#[async_trait]
impl PagerSource for ParentPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;

    type Entity = Entity;

    type Data = (i32, chrono::NaiveDateTime);

    const TYPE_NUMBER: u8 = 8;

    async fn filter(auth: &Auth, data: &Self::Data) -> Result<Select<Self::Entity>, Error> {
        let db = DB.get().unwrap();
        let parent: problem::IdModel = problem::Entity::related_read_by_id(auth, data.0)
            .into_partial_model()
            .one(db)
            .await?
            .ok_or(Error::NotInDB(problem::Entity::DEBUG_NAME))?;

        Ok(parent.upgrade().find_related(Entity))
    }
}

#[async_trait]
impl PagerSortSource<PartialModel> for ParentPagerTrait {
    fn sort_col(_data: &Self::Data) -> impl ColumnTrait {
        Column::UploadAt
    }
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send {
        data.1
    }
    fn save_val(data: &mut Self::Data, model: &PartialModel) {
        data.1 = model.upload_at
    }
}

pub type ParentPaginator = ColPager<ParentPagerTrait, PartialModel>;

pub struct ColPagerTrait;

#[async_trait]
impl PagerSource for ColPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;

    type Entity = Entity;

    type Data = chrono::NaiveDateTime;

    const TYPE_NUMBER: u8 = 8;

    async fn filter(auth: &Auth, _data: &Self::Data) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth)
    }
}

#[async_trait]
impl PagerSortSource<PartialModel> for ColPagerTrait {
    fn sort_col(data: &Self::Data) -> impl ColumnTrait {
        Column::UploadAt
    }
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send {
        data.clone()
    }
    fn save_val(data: &mut Self::Data, model: &PartialModel) {
        *data = model.upload_at
    }
}

pub type ColPaginator = ColPager<ColPagerTrait, PartialModel>;
