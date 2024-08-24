use crate::util::auth::RoleLv;
use tracing::instrument;

use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "chat")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub problem_id: i32,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
    pub message: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::problem::Entity",
        from = "Column::ProblemId",
        to = "super::problem::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Problem,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
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
    #[instrument(skip_all, level = "debug")]
    fn read_filter<S: QueryFilter + Send>(query: S, _: &Auth) -> Result<S, Error> {
        Ok(query)
    }
    #[instrument(skip_all, level = "debug")]
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        match auth.perm() >= RoleLv::Admin {
            true => Ok(query),
            false => Err(Error::RequirePermission(RoleLv::Admin)),
        }
    }
    fn writable(_: &Self::Model, auth: &Auth) -> bool {
        auth.perm() >= RoleLv::Admin
    }
}

#[async_trait]
impl Reflect<Entity> for Model {
    fn get_id(&self) -> i32 {
        self.id
    }

    async fn all(query: Select<Entity>, db: &DatabaseConnection) -> Result<Vec<Self>, Error> {
        query.all(db).await.map_err(Into::<Error>::into)
    }
}

pub struct ParentPagerTrait;

impl PagerData for ParentPagerTrait {
    type Data = (i32, chrono::NaiveDateTime);
}

#[async_trait]
impl Source for ParentPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;
    type Entity = Entity;
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
impl SortSource<Model> for ParentPagerTrait {
    fn sort_col(_data: &Self::Data) -> impl ColumnTrait {
        Column::CreateAt
    }
    fn get_val(data: &Self::Data) -> impl Into<Value> + Clone + Send {
        data.1
    }
    fn save_val(data: &mut Self::Data, model: &Model) {
        data.1 = model.create_at
    }
}

type ParentPaginator = UninitPaginator<ColumnPaginator<ParentPagerTrait, Model>>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Paginator(ParentPaginator);

impl WithAuthTrait for Paginator {}

impl Paginator {
    pub fn new(parent: i32, start_from_end: bool) -> Self {
        Self(ParentPaginator::new(
            (parent, Default::default()),
            start_from_end,
        ))
    }
}

impl<'a, 'b> WithDB<'a, WithAuth<'b, Paginator>> {
    pub async fn fetch(&mut self, size: u64, offset: i64) -> Result<Vec<Model>, Error> {
        let db = self.0;
        let auth = self.1 .0;
        self.1 .1 .0.fetch(size, offset, auth, db).await
    }
    pub async fn remain(&self) -> Result<u64, Error> {
        let db = self.0;
        let auth = self.1 .0;
        self.1 .1 .0.remain(auth, db).await
    }
}
