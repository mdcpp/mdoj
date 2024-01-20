use crate::grpc::backend::ContestSortBy;

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
    pub update_at: chrono::NaiveDateTime,
    pub create_at: chrono::NaiveDateTime,
}

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

// impl Related<Entity> for Entity {
//     fn to() -> RelationDef {
//         Relation::Public.def()
//     }
// }

impl ActiveModelBehavior for ActiveModel {}

impl super::DebugName for Entity {
    const DEBUG_NAME: &'static str = "contest";
}

#[tonic::async_trait]
impl ParentalTrait for Entity {
    const COL_ID: Column = Column::Id;

    fn related_filter(auth: &Auth) -> Select<Entity> {
        match user::Model::new_with_auth(auth) {
            Some(user) => user
                .find_related(Entity)
                .join_as(
                    JoinType::FullOuterJoin,
                    Relation::Hoster.def().rev(),
                    Alias::new("own_contest"),
                )
                .join_as(
                    JoinType::FullOuterJoin,
                    user::Relation::PublicContest.def(),
                    Alias::new("user_contest_unused"),
                ),
            None => Entity::find().filter(Column::Public.eq(true)),
        }
        .distinct()
    }
}

impl super::Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.admin() {
                return Ok(query);
            }
            return Ok(query.filter(Column::Public.eq(true).or(Column::Hoster.eq(user_id))));
        }
        Ok(query.filter(Column::Public.eq(true)))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.admin() {
            return Ok(query);
        }
        if perm.super_user() {
            return Ok(query.filter(Column::Hoster.eq(user_id)));
        }
        Err(Error::NotInDB(Entity::DEBUG_NAME))
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

pub struct TextPagerTrait;

#[async_trait]
impl PagerSource for TextPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;

    type Entity = Entity;

    type Data = String;

    const TYPE_NUMBER: u8 = 4;

    async fn filter(auth: &Auth, data: &Self::Data) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth).map(|x| x.filter(Column::Title.like(data)))
    }
}

pub type TextPaginator = PkPager<TextPagerTrait, PartialModel>;

pub struct ColPagerTrait;

#[async_trait]
impl PagerSource for ColPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;

    type Entity = Entity;

    // FIXME: we need optional support
    type Data = (ContestSortBy, String);

    const TYPE_NUMBER: u8 = 8;

    async fn filter(auth: &Auth, _data: &Self::Data) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth)
    }
}

#[async_trait]
impl PagerSortSource<PartialModel> for ColPagerTrait {
    fn sort_col(data: &Self::Data) -> impl ColumnTrait {
        match data.0 {
            ContestSortBy::UpdateDate => Column::UpdateAt,
            ContestSortBy::CreateDate => Column::CreateAt,
            ContestSortBy::Begin => Column::Begin,
            ContestSortBy::End => Column::End,
        }
    }
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send {
        &data.1
    }
    fn save_val(data: &mut Self::Data, model: &PartialModel) {
        match data.0 {
            ContestSortBy::UpdateDate => data.1 = model.update_at.to_string(),
            ContestSortBy::CreateDate => data.1 = model.create_at.to_string(),
            ContestSortBy::Begin => data.1 = model.begin.to_string(),
            ContestSortBy::End => data.1 = model.end.to_string(),
        }
    }
}

pub type ColPaginator = ColPager<ColPagerTrait, PartialModel>;
