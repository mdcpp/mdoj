use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::chat_set_server::*;
use crate::grpc::backend::*;
use crate::grpc::into_chrono;
use crate::grpc::into_prost;
use entity::{chat::*, *};
use sea_orm::QueryOrder;

impl Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, _: &Auth) -> Result<S, Error> {
        Ok(query)
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() || perm.can_manage_chat() {
                return Ok(query);
            }
        }
        Err(Error::Unauthenticated)
    }
}

impl From<i32> for ChatId {
    fn from(value: i32) -> Self {
        ChatId { id: value }
    }
}

impl From<ChatId> for i32 {
    fn from(value: ChatId) -> Self {
        value.id
    }
}

impl From<Model> for ChatInfo {
    fn from(value: Model) -> Self {
        ChatInfo {
            id: value.id.into(),
            user_id: value.user_id.into(),
            problem_id: value.problem_id.into(),
            create_at: into_prost(value.create_at),
            message: value.message,
        }
    }
}

#[tonic::async_trait]
impl ChatSet for Arc<Server> {
    async fn create(&self, req: Request<CreateChatRequest>) -> Result<Response<ChatId>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, _) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check_i32(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(model, req, message);

        let model = model.save(db).await.map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id.clone().unwrap());

        tracing::debug!(id = model.id.clone().unwrap());
        self.metrics.chat.add(1, &[]);

        Ok(Response::new(model.id.unwrap().into()))
    }

    async fn remove(&self, req: Request<ChatId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        self.metrics.chat.add(-1, &[]);
        tracing::debug!(id = req.id, "chat_remove");

        Ok(Response::new(()))
    }

    async fn list_by_problem(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListChatResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::ParentId(ppk) => {
                tracing::debug!(id = ppk);
                Pager::parent_search(ppk)
            }
            list_by_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<problem::Entity, Entity>>::from_raw(
                    old.session,
                    self,
                )?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw(self);

        Ok(Response::new(ListChatResponse { list, next_session }))
    }
}
