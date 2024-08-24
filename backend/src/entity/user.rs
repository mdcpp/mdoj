use self::util::paginator::Remain;
use grpc::backend::list_user_request::Sort;
use sea_orm::{QueryOrder, QuerySelect};
use tracing::instrument;

use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub permission: i32,
    pub score: i64,
    pub username: String,
    #[sea_orm(column_type = "Blob")]
    pub password: Vec<u8>,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
}

impl Model {
    /// create new model with only id and permission, useful for query
    ///
    /// Be careful never save it
    pub fn new_with_auth(auth: &Auth) -> Option<Self> {
        auth.assume_login().ok().map(|(id, permission)| Self {
            id,
            permission: permission as i32,
            score: Default::default(),
            username: Default::default(),
            password: Default::default(),
            create_at: Default::default(),
        })
    }
}

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
    #[sea_orm(has_many = "super::testcase::Entity")]
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

impl Related<announcement::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Announcement.def()
    }
}

impl Related<education::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Education.def()
    }
}

impl Related<problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl Related<submit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Submit.def()
    }
}

impl Related<testcase::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Test.def()
    }
}

impl Related<token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Token.def()
    }
}

impl Related<user_contest::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserContest.def()
    }
}

impl Related<contest::Entity> for Entity {
    fn to() -> RelationDef {
        user_contest::Relation::Contest.def()
    }
    fn via() -> Option<RelationDef> {
        Some(user_contest::Relation::User.def().rev())
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
            user_contest::Relation::Contest.def(),
            contest::Relation::Problem.def(),
        ]
    }
}

impl Filter for Entity {
    #[instrument(skip_all, level = "debug")]
    fn read_filter<S: QueryFilter + Send>(query: S, _: &Auth) -> Result<S, Error> {
        Ok(query)
    }

    #[instrument(skip_all, level = "debug")]
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.assume_login()?;
        perm.admin()?;
        Ok(match perm == RoleLv::Root {
            true => query,
            false => query.filter(
                Column::Permission
                    .lt(RoleLv::Admin as i32)
                    .or(Column::Id.eq(user_id)),
            ),
        })
    }
    fn writable(model: &Self::Model, auth: &Auth) -> bool {
        Some(model.id) == auth.user_id() || model.permission > auth.perm() as i32
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
        Entity::read_filter(Entity::find(), auth).map(|x| x.filter(Column::Username.contains(data)))
    }
}

type TextPaginator = PrimaryKeyPaginator<TextPagerTrait, Model>;

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
impl SortSource<Model> for ColPagerTrait {
    fn sort_col(data: &Self::Data) -> impl ColumnTrait {
        match data.0 {
            Sort::Score => Column::Score,
            Sort::CreateDate => Column::CreateAt,
        }
    }
    fn get_val(data: &Self::Data) -> impl Into<Value> + Clone + Send {
        &data.1
    }
    fn save_val(data: &mut Self::Data, model: &Model) {
        data.1 = match data.0 {
            Sort::Score => model.score.to_string(),
            Sort::CreateDate => model.create_at.to_string(),
        }
    }
}

type ColPaginator = ColumnPaginator<ColPagerTrait, Model>;

/// ParentPaginator (offset base)
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ParentPaginator {
    pub offset: u64,
    pub ppk: i32,
    pub start_from_end: bool,
}

pub struct ParentSource;

impl PagerData for ParentSource {
    type Data = (u64, i32);
}

fn to_order(raw: bool) -> sea_query::Order {
    match raw {
        true => sea_query::Order::Desc,
        false => sea_query::Order::Asc,
    }
}

#[async_trait]
impl PaginateRaw for ParentPaginator {
    type Source = ParentSource;
    type Reflect = Model;

    async fn fetch(
        &mut self,
        auth: &Auth,
        size: i64,
        offset: u64,
        db: &DatabaseConnection,
    ) -> Result<Vec<Self::Reflect>, Error> {
        let dir = size.is_negative();
        // check user is in contest(or admin)
        contest::Entity::read_by_id(self.ppk, auth)?
            .one(db)
            .await?
            .ok_or(Error::NotInDB)?;

        let result = user_contest::Entity::find()
            .filter(user_contest::Column::ContestId.eq(self.ppk))
            .order_by(
                user_contest::Column::Score,
                to_order(self.start_from_end ^ dir),
            )
            .offset(self.offset + offset)
            .limit(size.unsigned_abs())
            .all(db)
            .await?;

        let result: Vec<_> = result
            .load_one(Entity, db)
            .await?
            .into_iter()
            .flatten()
            .collect();

        self.offset += result.len() as u64;
        Ok(result)
    }
    async fn new_fetch(
        data: <Self::Source as PagerData>::Data,
        auth: &Auth,
        size: u64,
        offset: u64,
        abs_dir: bool,
        db: &DatabaseConnection,
    ) -> Result<(Self, Vec<Self::Reflect>), Error> {
        let ppk = data.1;
        // check user is in contest(or admin)
        contest::Entity::read_by_id(ppk, auth)?
            .one(db)
            .await?
            .ok_or(Error::NotInDB)?;

        let result = user_contest::Entity::find()
            .filter(user_contest::Column::ContestId.eq(data.0))
            .order_by(user_contest::Column::Score, to_order(abs_dir))
            .offset(offset)
            .limit(size)
            .all(db)
            .await?;

        let result: Vec<_> = result
            .load_one(Entity, db)
            .await?
            .into_iter()
            .flatten()
            .collect();

        let offset = offset + result.len() as u64;

        Ok((
            ParentPaginator {
                ppk,
                offset,
                start_from_end: abs_dir,
            },
            result,
        ))
    }
}

#[async_trait]
impl Remain for ParentPaginator {
    async fn remain(&self, auth: &Auth, db: &DatabaseConnection) -> Result<u64, Error> {
        contest::Entity::read_by_id(self.ppk, auth)?
            .one(db)
            .await?
            .ok_or(Error::NotInDB)?;

        let result = user_contest::Entity::find()
            .filter(user_contest::Column::ContestId.eq(self.ppk))
            .order_by(user_contest::Column::Score, to_order(self.start_from_end))
            .count(db)
            .await?;

        Ok(result.saturating_sub(self.offset))
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Paginator {
    Text(UninitPaginator<TextPaginator>),
    Parent(UninitPaginator<ParentPaginator>),
    Col(UninitPaginator<ColPaginator>),
}

impl WithAuthTrait for Paginator {}

impl Paginator {
    pub fn new_text(text: String, start_from_end: bool) -> Self {
        // FIXME: check dup text
        Self::Text(UninitPaginator::new(text, start_from_end))
    }
    pub fn new_sort(sort: Sort, start_from_end: bool) -> Self {
        Self::Col(UninitPaginator::new(
            (sort, Default::default()),
            start_from_end,
        ))
    }
    pub fn new_parent(parent: i32, start_from_end: bool) -> Self {
        Self::Parent(UninitPaginator::new(
            (Default::default(), parent),
            start_from_end,
        ))
    }
    pub fn new(start_from_end: bool) -> Self {
        Self::new_sort(Sort::Score, start_from_end)
    }
}

impl<'a, 'b> WithDB<'a, WithAuth<'b, Paginator>> {
    pub async fn fetch(&mut self, size: u64, offset: i64) -> Result<Vec<Model>, Error> {
        let db = self.0;
        let auth = self.1 .0;
        match &mut self.1 .1 {
            Paginator::Text(ref mut x) => x.fetch(size, offset, auth, db).await,
            Paginator::Parent(ref mut x) => x.fetch(size, offset, auth, db).await,
            Paginator::Col(ref mut x) => x.fetch(size, offset, auth, db).await,
        }
    }
    pub async fn remain(&self) -> Result<u64, Error> {
        let db = self.0;
        let auth = self.1 .0;
        match &self.1 .1 {
            Paginator::Text(x) => x.remain(auth, db).await,
            Paginator::Parent(x) => x.remain(auth, db).await,
            Paginator::Col(x) => x.remain(auth, db).await,
        }
    }
}
