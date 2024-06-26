use std::ops::Deref;

use chrono::Local;
use sea_orm::Statement;
use tracing::{instrument, Instrument};

use crate::union;
use grpc::backend::ContestSortBy;

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
pub struct PartialModel {
    pub id: i32,
    pub hoster: i32,
    pub begin: chrono::NaiveDateTime,
    pub end: chrono::NaiveDateTime,
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
    pub hoster: i32,
    pub public: bool,
}

impl IdModel {
    /// upgrade IdModel to Model to call find_related
    ///
    /// Be careful not to save it
    pub fn upgrade(self) -> Model {
        Model {
            id: self.id,
            hoster: self.hoster,
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

impl ActiveModelBehavior for ActiveModel {}

#[tonic::async_trait]
impl super::ParentalTrait<IdModel> for Entity {
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
                        [Column::Id, Column::Hoster, Column::Public, Column::Begin],
                        user.find_related(Entity),
                        Entity::find()
                            .filter(Column::Public.eq(true))
                            .filter(Column::Begin.lte(now)),
                        Entity::find().filter(Column::Hoster.eq(user.id))
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

impl super::Filter for Entity {
    #[instrument(skip_all, level = "debug")]
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.admin() {
                return Ok(query);
            }
            return Ok(query.filter(Column::Public.eq(true).or(Column::Hoster.eq(user_id))));
        }
        Ok(query.filter(Column::Public.eq(true)))
    }
    #[instrument(skip_all, level = "debug")]
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.admin() {
            return Ok(query);
        }
        if perm.super_user() {
            return Ok(query.filter(Column::Hoster.eq(user_id)));
        }
        Err(Error::NotInDB)
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
    const TYPE_NUMBER: u8 = 4;

    async fn filter(
        auth: &Auth,
        data: &Self::Data,
        _db: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth).map(|x| x.filter(Column::Title.contains(data)))
    }
}

pub type TextPaginator = PrimaryKeyPaginator<TextPagerTrait, PartialModel>;

pub struct ColPagerTrait;

impl PagerData for ColPagerTrait {
    type Data = (ContestSortBy, String);
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
    fn sort_col(data: &Self::Data) -> impl ColumnTrait {
        match data.0 {
            ContestSortBy::UpdateDate => Column::UpdateAt,
            ContestSortBy::CreateDate => Column::CreateAt,
            ContestSortBy::Begin => Column::Begin,
            ContestSortBy::End => Column::End,
            ContestSortBy::Public => Column::Public,
        }
    }
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send {
        &data.1
    }
    fn save_val(data: &mut Self::Data, model: &PartialModel) {
        data.1 = match data.0 {
            ContestSortBy::UpdateDate => model.update_at.to_string(),
            ContestSortBy::CreateDate => model.create_at.to_string(),
            ContestSortBy::Begin => model.begin.to_string(),
            ContestSortBy::End => model.end.to_string(),
            ContestSortBy::Public => model.public.to_string(),
        }
    }
}

pub type ColPaginator = ColumnPaginator<ColPagerTrait, PartialModel>;
