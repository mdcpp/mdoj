use super::*;
use crate::util::auth::{Auth, RoleLv};
use crate::util::error::Error;
use sea_orm::entity::prelude::*;
use tonic::async_trait;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "token")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    #[sea_orm(column_type = "Blob")]
    pub rand: Vec<u8>,
    pub permission: i32,
    #[sea_orm(column_type = "Time")]
    pub expiry: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl super::Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, role) = auth.assume_login()?;
        Ok(match role {
            RoleLv::Root => query,
            _ => query.filter(Column::UserId.eq(user_id)),
        })
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        Self::read_filter(query, auth)
    }
    fn writable(model: &Self::Model, auth: &Auth) -> bool {
        auth.perm() == RoleLv::Root
    }
}

#[async_trait]
impl Reflect<Entity> for Model {
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

struct ColPagerTrait;

impl PagerData for ColPagerTrait {
    type Data = chrono::NaiveDateTime;
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
impl SortSource<Model> for ColPagerTrait {
    fn sort_col(data: &Self::Data) -> impl ColumnTrait {
        Column::Expiry
    }
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send {
        data.to_string()
    }
    fn save_val(data: &mut Self::Data, model: &Model) {
        *data = model.expiry;
    }
}

type DefaultPaginator = UninitPaginator<ColumnPaginator<ColPagerTrait, Model>>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Paginator(DefaultPaginator);

impl WithAuthTrait for Paginator {}

impl Paginator {
    pub fn new(start_from_end: bool) -> Self {
        Self(DefaultPaginator::new(Default::default(), start_from_end))
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
