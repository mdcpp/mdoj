use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "education")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(nullable)]
    pub problem_id: Option<i32>,
    pub user_id: i32,
    pub tags: String,
    pub title: String,
    pub content: String,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct PartialModel {
    pub id: i32,
    pub problem_id: Option<i32>,
    pub user_id: i32,
    pub tags: String,
    pub title: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::problem::Entity",
        from = "Column::ProblemId",
        to = "super::problem::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Problem,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
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

pub struct PagerTrait;

impl PagerData for PagerTrait {
    type Data = ();
}

#[async_trait]
impl Source for PagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;
    type Entity = Entity;
    const TYPE_NUMBER: u8 = 4;

    async fn filter(
        auth: &Auth,
        _data: &Self::Data,
        _db: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth)
    }
}

type DefaultPaginator = UninitPaginator<PrimaryKeyPaginator<PagerTrait, PartialModel>>;

pub struct TextPagerTrait;

impl PagerData for TextPagerTrait {
    type Data = String;
}

#[async_trait]
impl Source for TextPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;
    type Entity = Entity;
    const TYPE_NUMBER: u8 = 4;

    async fn filter(
        auth: &Auth,
        data: &Self::Data,
        _db: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth).map(|x| x.filter(Column::Title.like(data)))
    }
}

type TextPaginator = UninitPaginator<PrimaryKeyPaginator<TextPagerTrait, PartialModel>>;

pub struct ParentPagerTrait;

impl PagerData for ParentPagerTrait {
    type Data = i32;
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
        let parent: problem::IdModel = problem::Entity::related_read_by_id(auth, *data, db).await?;
        Ok(parent.upgrade().find_related(Entity))
    }
}

type ParentPaginator = UninitPaginator<PrimaryKeyPaginator<ParentPagerTrait, PartialModel>>;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Paginator {
    Text(TextPaginator),
    Parent(ParentPaginator),
    Default(DefaultPaginator),
}

impl WithAuthTrait for Paginator {}

impl Paginator {
    pub fn new_text(text: String, start_from_end: bool) -> Self {
        Self::Text(TextPaginator::new(text, start_from_end))
    }
    pub fn new_parent(parent: i32, start_from_end: bool) -> Self {
        Self::Parent(ParentPaginator::new(parent, start_from_end))
    }
    pub fn new(start_from_end: bool) -> Self {
        Self::Default(DefaultPaginator::new((), start_from_end))
    }
}

impl<'a, 'b> WithDB<'a, WithAuth<'b, Paginator>> {
    pub async fn fetch(&mut self, size: u64, offset: i64) -> Result<Vec<PartialModel>, Error> {
        let db = self.0;
        let auth = self.1 .0;
        match &mut self.1 .1 {
            Paginator::Text(ref mut x) => x.fetch(size, offset, auth, db).await,
            Paginator::Parent(ref mut x) => x.fetch(size, offset, auth, db).await,
            Paginator::Default(ref mut x) => x.fetch(size, offset, auth, db).await,
        }
    }
    pub async fn remain(&self) -> Result<u64, Error> {
        let db = self.0;
        let auth = self.1 .0;
        match &self.1 .1 {
            Paginator::Text(x) => x.remain(auth, db).await,
            Paginator::Parent(x) => x.remain(auth, db).await,
            Paginator::Default(x) => x.remain(auth, db).await,
        }
    }
}
