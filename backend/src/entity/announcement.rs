use tracing::instrument;

use crate::grpc::backend::AnnouncementSortBy;

use super::*;

pub static NAME: &str = "announcement";

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "announcement")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub title: String,
    pub content: String,
    #[sea_orm(nullable)]
    pub contest_id: Option<i32>,
    pub user_id: i32,
    pub public: bool,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time", on_update = "current_timestamp")]
    pub update_at: chrono::NaiveDateTime,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct PartialModel {
    pub id: i32,
    pub title: String,
    pub contest_id: Option<i32>,
    pub user_id: i32,
    pub public: bool,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time", on_update = "current_timestamp")]
    pub update_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::contest::Entity",
        from = "Column::ContestId",
        to = "super::contest::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Contest,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    User,
}

impl Related<super::contest::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Contest.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl super::Filter for Entity {
    #[instrument(skip_all, level = "debug")]
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.admin() {
                return Ok(query);
            }
            return Ok(query.filter(Column::Public.eq(true).or(Column::UserId.eq(user_id))));
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
            return Ok(query.filter(Column::UserId.eq(user_id)));
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
        _: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth)
    }
}

pub type Paginator = PrimaryKeyPaginator<PagerTrait, PartialModel>;

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
        _: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth).map(|x| x.filter(Column::Title.contains(data)))
    }
}

pub type TextPaginator = PrimaryKeyPaginator<TextPagerTrait, PartialModel>;

pub struct ParentPagerTrait;

impl PagerData for ParentPagerTrait {
    type Data = (i32, chrono::NaiveDateTime);
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
        let parent: contest::IdModel =
            contest::Entity::related_read_by_id(auth, data.0, db).await?;

        Ok(parent.upgrade().find_related(Entity))
    }
}

#[async_trait]
impl SortSource<PartialModel> for ParentPagerTrait {
    fn sort_col(_data: &Self::Data) -> impl ColumnTrait {
        Column::UpdateAt
    }
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send {
        data.1
    }
    fn save_val(data: &mut Self::Data, model: &PartialModel) {
        data.1 = model.update_at
    }
}

pub type ParentPaginator = ColumnPaginator<ParentPagerTrait, PartialModel>;

pub struct ColPagerTrait;

impl PagerData for ColPagerTrait {
    type Data = (AnnouncementSortBy, String);
}

#[async_trait]
impl Source for ColPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;
    type Entity = Entity;
    const TYPE_NUMBER: u8 = 8;

    async fn filter(
        auth: &Auth,
        _data: &Self::Data,
        _: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth)
    }
}

#[async_trait]
impl SortSource<PartialModel> for ColPagerTrait {
    fn sort_col(data: &Self::Data) -> impl ColumnTrait {
        match data.0 {
            AnnouncementSortBy::UpdateDate => Column::UpdateAt,
            AnnouncementSortBy::CreateDate => Column::CreateAt,
            AnnouncementSortBy::Public => Column::Public,
        }
    }
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send {
        &data.1
    }
    fn save_val(data: &mut Self::Data, model: &PartialModel) {
        data.1 = match data.0 {
            AnnouncementSortBy::UpdateDate => model.update_at.to_string(),
            AnnouncementSortBy::CreateDate => model.create_at.to_string(),
            AnnouncementSortBy::Public => model.public.to_string(),
        }
    }
}

pub type ColPaginator = ColumnPaginator<ColPagerTrait, PartialModel>;
