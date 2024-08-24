use super::*;
use crate::union;
use grpc::backend::list_problem_request::Sort;
use sea_orm::{ActiveValue, QuerySelect, Statement};
use sea_query::{JoinType};
use spin::{RwLock, RwLockReadGuard};
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{instrument, Instrument};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "problem")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    #[sea_orm(nullable)]
    pub contest_id: Option<i32>,
    pub accept_count: u32,
    pub submit_count: u32,
    #[sea_orm(column_type = "Float")]
    pub ac_rate: f32,
    pub memory: i64,
    pub time: i64,
    pub difficulty: u32,
    pub public: bool,
    pub title: String,
    pub content: String,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time", on_update = "current_timestamp")]
    pub update_at: chrono::NaiveDateTime,
    pub match_rule: i32,
    pub order: f32,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct PartialModel {
    pub id: i32,
    pub user_id: i32,
    pub contest_id: Option<i32>,
    pub submit_count: u32,
    pub ac_rate: f32,
    pub difficulty: u32,
    pub public: bool,
    pub title: String,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time", on_update = "current_timestamp")]
    pub update_at: chrono::NaiveDateTime,
    pub order: f32,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct IdModel {
    pub id: i32,
    pub user_id: i32,
    pub contest_id: Option<i32>,
    pub public: bool,
}

impl IdModel {
    /// create new model with only ids(foreign and primary), useful for query
    ///
    /// Be careful never save it
    pub fn upgrade(self) -> Model {
        Model {
            id: self.id,
            user_id: self.user_id,
            contest_id: self.contest_id,
            accept_count: Default::default(),
            submit_count: Default::default(),
            ac_rate: Default::default(),
            memory: Default::default(),
            time: Default::default(),
            difficulty: Default::default(),
            public: self.public,
            title: Default::default(),
            content: Default::default(),
            create_at: Default::default(),
            update_at: Default::default(),
            match_rule: Default::default(),
            order: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::chat::Entity")]
    Chat,
    #[sea_orm(
        belongs_to = "super::contest::Entity",
        from = "Column::ContestId",
        to = "super::contest::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Contest,
    #[sea_orm(has_many = "super::education::Entity")]
    Education,
    #[sea_orm(has_many = "super::submit::Entity")]
    Submit,
    #[sea_orm(has_many = "super::tag_problem::Entity")]
    TagProblem,
    #[sea_orm(has_many = "super::testcase::Entity")]
    Testcase,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    User,
}

impl Related<education::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Education.def()
    }
}

impl Related<submit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Submit.def()
    }
}

impl Related<chat::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chat.def()
    }
}

impl Related<testcase::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Testcase.def()
    }
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<contest::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Contest.def()
    }
}

impl Related<tag::Entity> for Entity {
    fn to() -> RelationDef {
        tag_problem::Relation::Tag.def()
    }
    fn via() -> Option<RelationDef> {
        Some(tag_problem::Relation::Problem.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub async fn insert_tag<C: ConnectionTrait + 'static + Send>(
    db: Arc<C>,
    tags: impl Iterator<Item = String>,
    id: i32,
) -> Result<(), DbErr> {
    let tags = tags.collect::<Vec<_>>();
    let mut tag_ids = Vec::new();
    let mut set = JoinSet::new();
    for tag in tags.clone() {
        let db = db.clone();
        set.spawn(async move {
            tag::ActiveModel {
                name: ActiveValue::set(tag),
                ..Default::default()
            }
            .save(db.deref())
            .await
            .map(|x| Some(x.id.unwrap()))
        });
    }
    for tag in tags {
        let db = db.clone();
        set.spawn(async move {
            tag::Entity::find()
                .filter(tag::Column::Name.eq(tag))
                .one(db.deref())
                .await
                .map(|x| x.map(|x| x.id))
        });
    }
    while let Some(Ok(x)) = set.join_next().await {
        if let Ok(Some(x)) = x {
            tag_ids.push(x);
        }
    }

    tag_ids.sort_unstable();
    tag_ids.dedup();

    tag_problem::Entity::delete_many()
        .filter(tag_problem::Column::ProblemId.eq(id))
        .exec(db.deref())
        .await?;

    let mut set = JoinSet::new();
    for tag_id in tag_ids {
        let db = db.clone();
        set.spawn(async move {
            tag_problem::ActiveModel {
                tag_id: ActiveValue::set(tag_id),
                problem_id: ActiveValue::set(id),
                ..Default::default()
            }
            .save(db.deref())
            .await
        });
    }
    while let Some(Ok((x))) = set.join_next().await {
        x?;
    }
    Ok(())
}

#[async_trait]
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

                    union!(
                        [
                            Column::Id,
                            Column::UserId,
                            Column::ContestId,
                            Column::Public
                        ],
                        user.find_related(Entity),
                        Entity::find().filter(Column::Public.eq(true))
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
                    .or(Column::UserId.eq(auth.user_id().unwrap())),
            ),
            RoleLv::Admin | RoleLv::Root => query,
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

struct PagerTrait;

struct TextPagerTrait;

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

type TextPaginator = UninitPaginator<PrimaryKeyPaginator<TextPagerTrait, PartialModel>>;

struct ParentPagerTrait;

impl PagerData for ParentPagerTrait {
    type Data = (i32, f32);
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
        Column::Order
    }
    fn get_val(data: &Self::Data) -> impl Into<sea_orm::Value> + Clone + Send {
        data.1
    }
    fn save_val(data: &mut Self::Data, model: &PartialModel) {
        data.1 = model.order
    }
}

type ParentPaginator = UninitPaginator<ColumnPaginator<ParentPagerTrait, PartialModel>>;

struct ColPagerTrait;

impl PagerData for ColPagerTrait {
    type Data = (Sort, String);
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
            Sort::UpdateDate => Column::UpdateAt,
            Sort::CreateDate => Column::CreateAt,
            Sort::AcRate => Column::AcRate,
            Sort::SubmitCount => Column::SubmitCount,
            Sort::Difficulty => Column::Difficulty,
            Sort::Order => Column::Order,
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
            Sort::AcRate => model.ac_rate.to_string(),
            Sort::SubmitCount => model.submit_count.to_string(),
            Sort::Difficulty => model.difficulty.to_string(),
            Sort::Order => model.order.to_string(),
            Sort::Public => model.public.to_string(),
        }
    }
}

type ColPaginator = UninitPaginator<ColumnPaginator<ColPagerTrait, PartialModel>>;

pub struct TagPagerTrait;

impl PagerData for TagPagerTrait {
    type Data = Vec<String>;
}

#[async_trait]
impl Source for TagPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;
    type Entity = Entity;
    const TYPE_NUMBER: u8 = 0;

    async fn filter(
        auth: &Auth,
        data: &Self::Data,
        db: &DatabaseConnection,
    ) -> Result<Select<Self::Entity>, Error> {
        let len = data.len();
        let query = problem::Entity::find()
            .join(JoinType::Join, problem::Relation::TagProblem.def())
            .join(JoinType::Join, tag::Relation::TagProblem.def().rev())
            .filter(tag::Column::Name.is_in(data))
            .group_by(problem::Column::Id)
            .having(Expr::col(tag::Column::Name).count_distinct().eq(len as i32));
        Entity::read_filter(query, auth)
    }
}

type TagPaginator = UninitPaginator<PrimaryKeyPaginator<TagPagerTrait, PartialModel>>;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Paginator {
    Text(TextPaginator),
    Parent(ParentPaginator),
    Col(ColPaginator),
    Tag(TagPaginator),
}

impl WithAuthTrait for Paginator {}

impl Paginator {
    pub fn new_text(text: String, start_from_end: bool) -> Self {
        // FIXME: check dup text
        Self::Text(TextPaginator::new(text, start_from_end))
    }
    pub fn new_sort(sort: Sort, start_from_end: bool) -> Self {
        Self::Col(ColPaginator::new(
            (sort, Default::default()),
            start_from_end,
        ))
    }
    pub fn new_parent(parent: i32, start_from_end: bool) -> Self {
        Self::Parent(ParentPaginator::new(
            (parent, Default::default()),
            start_from_end,
        ))
    }
    pub fn new_tag(tag: impl Iterator<Item = String>, start_from_end: bool) -> Self {
        Self::Tag(TagPaginator::new(tag.collect(), start_from_end))
    }
    pub fn new(start_from_end: bool) -> Self {
        Self::new_sort(Sort::SubmitCount, start_from_end)
    }
}

impl<'a, 'b> WithDB<'a, WithAuth<'b, Paginator>> {
    pub async fn fetch(&mut self, size: u64, offset: i64) -> Result<Vec<PartialModel>, Error> {
        let db = self.0;
        let auth = self.1 .0;
        match &mut self.1 .1 {
            Paginator::Text(ref mut x) => x.fetch(size, offset, auth, db).await,
            Paginator::Parent(ref mut x) => x.fetch(size, offset, auth, db).await,
            Paginator::Col(ref mut x) => x.fetch(size, offset, auth, db).await,
            Paginator::Tag(ref mut x) => x.fetch(size, offset, auth, db).await,
        }
    }
    pub async fn remain(&self) -> Result<u64, Error> {
        let db = self.0;
        let auth = self.1 .0;
        match &self.1 .1 {
            Paginator::Text(x) => x.remain(auth, db).await,
            Paginator::Parent(x) => x.remain(auth, db).await,
            Paginator::Col(x) => x.remain(auth, db).await,
            Paginator::Tag(x) => x.remain(auth, db).await,
        }
    }
}
