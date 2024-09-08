use std::ops::Deref;

use crate::union;
use chrono::Local;
use grpc::backend::list_contest_request::Sort;
use sea_orm::Statement;
use tracing::{instrument, Instrument};

use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "contest")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub host: i32,
    #[sea_orm(column_type = "Time", nullable)]
    pub begin: Option<chrono::NaiveDateTime>,
    #[sea_orm(column_type = "Time", nullable)]
    pub end: Option<chrono::NaiveDateTime>,
    pub title: String,
    pub content: String,
    pub tags: String,
    #[sea_orm(column_type = "Blob", nullable)]
    pub password: Option<Vec<u8>>,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time", on_update = "current_timestamp")]
    pub update_at: chrono::NaiveDateTime,
    pub public: bool,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct PartialModel {
    pub id: i32,
    pub host: i32,
    pub begin: Option<chrono::NaiveDateTime>,
    pub end: Option<chrono::NaiveDateTime>,
    pub title: String,
    pub password: Option<Vec<u8>>,
    pub public: bool,
    #[sea_orm(column_type = "Time", on_update = "current_timestamp")]
    pub update_at: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
}

/// A partial model with only enough information to do `ParentalFilter`
#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct IdModel {
    pub id: i32,
    pub host: i32,
    pub public: bool,
}

impl IdModel {
    /// upgrade IdModel to Model to call find_related
    ///
    /// Be careful not to save it
    pub fn upgrade(self) -> Model {
        Model {
            id: self.id,
            host: self.host,
            begin: Default::default(),
            end: Default::default(),
            title: Default::default(),
            content: Default::default(),
            tags: Default::default(),
            password: Default::default(),
            create_at: Default::default(),
            update_at: Default::default(),
            public: self.public,
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::announcement::Entity")]
    Announcement,
    #[sea_orm(has_many = "super::problem::Entity")]
    Problem,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::Host",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    User,
    #[sea_orm(has_many = "super::user_contest::Entity")]
    UserContest,
}

impl Related<announcement::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Announcement.def()
    }
}

impl Related<user_contest::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserContest.def()
    }
}

impl Related<problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        user_contest::Relation::User.def()
    }
    fn via() -> Option<RelationDef> {
        Some(user_contest::Relation::Contest.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[tonic::async_trait]
impl ParentalTrait<IdModel> for Entity {
    #[instrument(skip_all, level = "info")]
    async fn related_read_by_id(
        auth: &Auth,
        id: i32,
        db: &DatabaseConnection,
    ) -> Result<IdModel, Error> {
        match user::Model::new_with_auth(auth) {
            Some(user) => {
                let (query, param) = {
                    let builder = db.get_database_backend().get_query_builder();
                    let now = Local::now().naive_local();

                    union!(
                        [Column::Id, Column::Host, Column::Public, Column::Begin],
                        user.find_related(Entity),
                        Entity::find().filter(Column::Public.eq(true).and(Column::Begin.lte(now))),
                        Entity::find().filter(Column::Host.eq(user.id))
                    )
                    .and_where(Column::Id.eq(id))
                    .build_any(builder.deref())
                };

                IdModel::find_by_statement(Statement::from_sql_and_values(
                    db.get_database_backend(),
                    query,
                    param,
                ))
                .one(db)
                .in_current_span()
                .await?
                .ok_or(Error::NotInDB)
            }
            None => Entity::find_by_id(id)
                .filter(Column::Public.eq(true))
                .into_partial_model()
                .one(db)
                .in_current_span()
                .await?
                .ok_or(Error::NotInDB),
        }
    }
}

impl Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        Ok(match auth.perm() {
            RoleLv::Guest => query.filter(Column::Public.eq(true)),
            RoleLv::User | RoleLv::Super => query.filter(
                Column::Public
                    .eq(true)
                    .or(Column::Host.eq(auth.user_id().unwrap())),
            ),
            RoleLv::Admin | RoleLv::Root => query,
        })
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.assume_login()?;
        Ok(match perm {
            RoleLv::Admin | RoleLv::Root => query,
            _ => query.filter(Column::Host.eq(user_id)),
        })
    }
    fn writable(model: &Self::Model, auth: &Auth) -> bool {
        auth.perm() >= RoleLv::Admin
            || (Some(model.host) == auth.user_id() && auth.perm() != RoleLv::User)
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

pub struct TextPagerTrait;

impl PagerData for TextPagerTrait {
    type Data = String;
}

#[async_trait]
impl Source for TextPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;
    type Entity = Entity;
    async fn filter(
        auth: &Auth,
        data: &Self::Data,
        _db: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth).map(|x| x.filter(Column::Title.contains(data)))
    }
}

type TextPaginator = UninitPaginator<PrimaryKeyPaginator<TextPagerTrait, PartialModel>>;

pub struct ColPagerTrait;

impl PagerData for ColPagerTrait {
    type Data = (Sort, String);
}

#[async_trait]
impl Source for ColPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;
    type Entity = Entity;
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
    fn sort_col(data: &Self::Data) -> impl ColumnTrait {
        match data.0 {
            Sort::UpdateDate => Column::UpdateAt,
            Sort::CreateDate => Column::CreateAt,
            Sort::Begin => Column::Begin,
            Sort::End => Column::End,
            Sort::Public => Column::Public,
        }
    }
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send {
        &data.1
    }
    fn save_val(data: &mut Self::Data, model: &PartialModel) {
        data.1 = match data.0 {
            Sort::UpdateDate => model.update_at.to_string(),
            Sort::CreateDate => model.create_at.to_string(),
            Sort::Begin => model.begin.unwrap().to_string(),
            Sort::End => model.end.unwrap().to_string(),
            Sort::Public => model.public.to_string(),
        }
    }
}

type ColPaginator = UninitPaginator<ColumnPaginator<ColPagerTrait, PartialModel>>;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Paginator {
    Text(TextPaginator),
    Col(ColPaginator),
}

impl WithAuthTrait for Paginator {}

impl Paginator {
    pub fn new_text(text: String, start_from_end: bool) -> Self {
        Self::Text(TextPaginator::new(text, start_from_end))
    }
    pub fn new_sort(sort: Sort, start_from_end: bool) -> Self {
        Self::Col(ColPaginator::new(
            (sort, Default::default()),
            start_from_end,
        ))
    }
    pub fn new(start_from_end: bool) -> Self {
        Self::new_sort(Sort::CreateDate, start_from_end)
    }
}

impl<'a, 'b> WithDB<'a, WithAuth<'b, Paginator>> {
    pub async fn fetch(&mut self, size: u64, offset: i64) -> Result<Vec<PartialModel>, Error> {
        let db = self.0;
        let auth = self.1 .0;
        match &mut self.1 .1 {
            Paginator::Text(ref mut x) => x.fetch(size, offset, auth, db).await,
            Paginator::Col(ref mut x) => x.fetch(size, offset, auth, db).await,
        }
    }
    pub async fn remain(&self) -> Result<u64, Error> {
        let db = self.0;
        let auth = self.1 .0;
        match &self.1 .1 {
            Paginator::Text(x) => x.remain(auth, db).await,
            Paginator::Col(x) => x.remain(auth, db).await,
        }
    }
}
