use super::*;

// FIXME: use partial model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "testcase")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    #[sea_orm(nullable)]
    pub problem_id: Option<i32>,
    #[sea_orm(column_type = "Blob")]
    pub input: Vec<u8>,
    #[sea_orm(column_type = "Blob")]
    pub output: Vec<u8>,
    pub score: u32,
    pub order: f32,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct PartialModel {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    #[sea_orm(nullable)]
    pub problem_id: Option<i32>,
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

impl Related<problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.assume_login()?;
        Ok(match perm {
            RoleLv::Admin | RoleLv::Root => query,
            _ => query.filter(Column::UserId.eq(user_id)),
        })
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.assume_login()?;
        match perm {
            RoleLv::Admin | RoleLv::Root => Ok(query),
            RoleLv::Super => Ok(query.filter(Column::UserId.eq(user_id))),
            _ => Err(Error::RequirePermission(RoleLv::Super)),
        }
    }
    fn writable(model: &Self::Model, auth: &Auth) -> bool {
        auth.perm() >= RoleLv::Admin
            || (Some(model.user_id) == auth.user_id() && auth.perm() != RoleLv::User)
    }
}

#[async_trait]
impl Reflect<Entity> for PartialModel {
    fn get_id(&self) -> i32 {
        self.id
    }

    async fn all(query: Select<Entity>, db: &DatabaseConnection) -> Result<Vec<Self>, Error> {
        query
            .into_model::<Self>()
            .all(db)
            .await
            .map_err(Into::<Error>::into)
    }
}

pub struct ParentPagerTrait;

impl PagerData for ParentPagerTrait {
    type Data = (i32, u32);
}

#[async_trait]
impl Source for ParentPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;
    type Entity = Entity;
    const TYPE_NUMBER: u8 = 8;

    async fn filter(
        auth: &Auth,
        data: &Self::Data,
        db: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        let parent: problem::IdModel =
            problem::Entity::related_read_by_id(auth, data.0, db).await?;
        Ok(parent.upgrade().find_related(Entity))
    }
}

#[async_trait]
impl SortSource<PartialModel> for ParentPagerTrait {
    fn sort_col(_data: &Self::Data) -> impl ColumnTrait {
        Column::Score
    }
    fn get_val(data: &Self::Data) -> impl Into<Value> + Clone + Send {
        data.1
    }
    fn save_val(data: &mut Self::Data, model: &PartialModel) {
        data.1 = model.score
    }
}

type ParentPaginator = UninitPaginator<ColumnPaginator<ParentPagerTrait, PartialModel>>;

pub struct ColPagerTrait;

impl PagerData for ColPagerTrait {
    type Data = u32;
}

#[async_trait]
impl Source for ColPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;
    type Entity = Entity;
    const TYPE_NUMBER: u8 = 8;

    async fn filter(
        auth: &Auth,
        _data: &Self::Data,
        _db: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth)
    }
}

#[async_trait]
impl SortSource<PartialModel> for ColPagerTrait {
    fn sort_col(_data: &Self::Data) -> impl ColumnTrait {
        Column::Score
    }
    fn get_val(data: &Self::Data) -> impl Into<Value> + Clone + Send {
        *data
    }
    fn save_val(data: &mut Self::Data, model: &PartialModel) {
        *data = model.score
    }
}

type DefaultPaginator = UninitPaginator<ColumnPaginator<ColPagerTrait, PartialModel>>;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Paginator {
    Parent(ParentPaginator),
    Default(DefaultPaginator),
}

impl WithAuthTrait for Paginator {}

impl Paginator {
    pub fn new_parent(parent: i32, start_from_end: bool) -> Self {
        Self::Parent(ParentPaginator::new(
            (parent, Default::default()),
            start_from_end,
        ))
    }
    pub fn new(start_from_end: bool) -> Self {
        Self::Default(DefaultPaginator::new(Default::default(), start_from_end))
    }
}

impl<'a, 'b> WithDB<'a, WithAuth<'b, Paginator>> {
    pub async fn fetch(&mut self, size: u64, offset: i64) -> Result<Vec<PartialModel>, Error> {
        let db = self.0;
        let auth = self.1 .0;
        match &mut self.1 .1 {
            Paginator::Parent(ref mut x) => x.fetch(size, offset, auth, db).await,
            Paginator::Default(ref mut x) => x.fetch(size, offset, auth, db).await,
        }
    }
    pub async fn remain(&self) -> Result<u64, Error> {
        let db = self.0;
        let auth = self.1 .0;
        match &self.1 .1 {
            Paginator::Parent(x) => x.remain(auth, db).await,
            Paginator::Default(x) => x.remain(auth, db).await,
        }
    }
}
