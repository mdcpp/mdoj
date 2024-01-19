use crate::grpc::backend::UserSortBy;

use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub permission: u32,
    pub score: u32,
    pub username: String,
    #[sea_orm(column_type = "Binary(BlobSize::Blob(None))")]
    pub password: Vec<u8>,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
}

impl Model {
    /// create new model with only id and permission, useful for query
    ///
    /// Be careful never save it
    pub fn new_with_auth(auth: &Auth) -> Option<Self> {
        auth.ok_or_default().ok().map(|(id, permission)| Self {
            id,
            permission: permission.0,
            score: Default::default(),
            username: Default::default(),
            password: Default::default(),
            create_at: Default::default(),
        })
    }
}

// #[derive(DeriveModel, FromQueryResult)]
// #[sea_orm(entity = "Entity")]
// pub struct IdUser {
//     pub id: i32,
// }

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::announcement::Entity")]
    Announcement,
    #[sea_orm(has_many = "super::chat::Entity")]
    Chat,
    #[sea_orm(has_many = "super::education::Entity")]
    Education,
    #[sea_orm(has_many = "super::problem::Entity")]
    Problem,
    #[sea_orm(has_many = "super::submit::Entity")]
    Submit,
    #[sea_orm(has_many = "super::test::Entity")]
    Test,
    #[sea_orm(has_many = "super::token::Entity")]
    Token,
    #[sea_orm(has_many = "super::user_contest::Entity")]
    UserContest,
    #[sea_orm(has_many = "super::contest::Entity")]
    OwnContest,
    #[sea_orm(
        has_many = "super::contest::Entity",
        on_condition = r#"super::contest::Column::Public.eq(true)"#
        condition_type = "any",
    )]
    PublicContest,
    #[sea_orm(
        has_many = "super::problem::Entity",
        on_condition = r#"super::problem::Column::Public.eq(true)"#
        condition_type = "any",
    )]
    PublicProblem,
}

impl Related<super::announcement::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Announcement.def()
    }
}

impl Related<super::education::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Education.def()
    }
}

impl Related<super::problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl Related<super::submit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Submit.def()
    }
}

impl Related<super::test::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Test.def()
    }
}

impl Related<super::token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Token.def()
    }
}

impl Related<super::user_contest::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserContest.def()
    }
}

impl Related<super::contest::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_contest::Relation::Contest.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::user_contest::Relation::User.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub struct UserToProblem;

impl Linked for UserToProblem {
    type FromEntity = Entity;

    type ToEntity = problem::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            Relation::UserContest.def(),
            user_contest::Entity::belongs_to(contest::Entity)
                .from(user_contest::Column::ContestId)
                .to(contest::Column::Id)
                .into(),
            contest::Relation::Problem.def(),
        ]
    }
}

impl super::DebugName for Entity {
    const DEBUG_NAME: &'static str = "user";
}

impl super::Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, _: &Auth) -> Result<S, Error> {
        Ok(query)
    }

    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() || perm.can_manage_user() {
            return Ok(query);
        }
        Ok(query.filter(Column::Id.eq(user_id)))
    }
}

#[async_trait]
impl PagerReflect<Entity> for Model {
    fn get_id(&self) -> i32 {
        self.id
    }

    async fn all(query: Select<Entity>) -> Result<Vec<Self>, Error> {
        let db = DB.get().unwrap();
        query.all(db).await.map_err(Into::<Error>::into)
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
        Entity::read_filter(Entity::find(), auth).map(|x| x.filter(Column::Username.like(data)))
    }
}

pub type TextPaginator = PkPager<TextPagerTrait, Model>;

pub struct ColPagerTrait;

#[async_trait]
impl PagerSource for ColPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;

    type Entity = Entity;

    type Data = (UserSortBy, String);

    const TYPE_NUMBER: u8 = 8;

    async fn filter(auth: &Auth, _data: &Self::Data) -> Result<Select<Self::Entity>, Error> {
        Entity::read_filter(Entity::find(), auth)
    }
}

#[async_trait]
impl PagerSortSource<Model> for ColPagerTrait {
    fn sort_col(data: &Self::Data) -> impl ColumnTrait {
        match data.0 {
            UserSortBy::Score => Column::Score,
            UserSortBy::CreateDate => Column::CreateAt,
        }
    }
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send {
        &data.1
    }
    fn save_val(data: &mut Self::Data, model: &Model) {
        data.1 = match data.0 {
            UserSortBy::Score => model.score.to_string(),
            UserSortBy::CreateDate => model.create_at.to_string(),
        }
    }
}

pub type ColPaginator = ColPager<ColPagerTrait, Model>;
