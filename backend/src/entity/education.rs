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
    const DEBUG_NAME: &'static str = "education";
}

impl super::Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.admin() {
                return Ok(query);
            }
            return Ok(query.filter(Column::UserId.eq(user_id)));
        }
        Err(Error::NotInDB(Entity::DEBUG_NAME))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.admin() {
            return Ok(query);
        }
        if perm.super_user() {
            return Ok(query.filter(Column::UserId.eq(user_id)));
        }
        Err(Error::NotInDB(Entity::DEBUG_NAME))
    }
}

#[async_trait]
impl PagerReflect<Entity> for PartialModel {
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

#[async_trait]
impl PagerSource for PagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;

    type Entity = Entity;

    type Data = ();

    const TYPE_NUMBER: u8 = 4;

    async fn filter(
        auth: &Auth,
        _data: &Self::Data,
        db: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth)
    }
}

pub type Paginator = PkPager<PagerTrait, PartialModel>;

pub struct TextPagerTrait;

#[async_trait]
impl PagerSource for TextPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;

    type Entity = Entity;

    type Data = String;

    const TYPE_NUMBER: u8 = 4;

    async fn filter(
        auth: &Auth,
        data: &Self::Data,
        db: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth).map(|x| x.filter(Column::Title.like(data)))
    }
}

pub type TextPaginator = PkPager<TextPagerTrait, PartialModel>;

pub struct ParentPagerTrait;

#[async_trait]
impl PagerSource for ParentPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;

    type Entity = Entity;

    type Data = i32;

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

pub type ParentPaginator = PkPager<ParentPagerTrait, PartialModel>;
