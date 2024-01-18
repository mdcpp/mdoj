use super::*;

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
    pub create_at: chrono::NaiveDateTime,
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

impl super::DebugName for Entity {
    const DEBUG_NAME: &'static str = "announcement";
}

impl super::Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.can_root() {
                return Ok(query);
            }
            return Ok(query.filter(Column::Public.eq(true).or(Column::UserId.eq(user_id))));
        }
        Ok(query.filter(Column::Public.eq(true)))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() {
            return Ok(query);
        }
        if perm.can_manage_announcement() {
            return Ok(query.filter(Column::UserId.eq(user_id)));
        }
        Err(Error::PermissionDeny("Can't write announcement"))
    }
}

pub struct ListPagerTrait;

impl PagerSource for ListPagerTrait {
    const ID: <Self::Entity as EntityTrait>::Column = Column::Id;

    type Entity = Entity;

    type Data = ();

    const TYPE_NUMBER: u8 = 1;

    fn filter(auth: &Auth, data: &Self::Data) -> Select<Self::Entity> {
        Entity::find().filter(Column::Public.eq(true))
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

pub type ListPager = PkPager<ListPagerTrait, PartialModel>;
