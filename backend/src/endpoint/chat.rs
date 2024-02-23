use super::tools::*;

use crate::grpc::backend::chat_set_server::*;
use crate::grpc::backend::*;

use crate::entity::chat::*;
use crate::grpc::into_prost;

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
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, _) = auth.ok_or_default()?;

        check_length!(LONG_ART_SIZE, req, message);

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<ChatId>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(model, req, message);

        let model = model
            .save(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        let id: ChatId = model.id.clone().unwrap().into();
        self.dup.store(user_id, uuid, id.clone());

        tracing::debug!(id = id.id, "chat_created");
        self.metrics.chat(1);

        Ok(Response::new(id))
    }

    async fn remove(&self, req: Request<ChatId>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let result = Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        if result.rows_affected == 0 {
            return Err(Error::NotInDB.into());
        }

        self.metrics.chat(-1);
        tracing::debug!(id = req.id, "chat_remove");

        Ok(Response::new(()))
    }

    async fn list_by_problem(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListChatResponse>, Status> {
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            list_by_request::Request::Create(create) => {
                tracing::debug!(id = create.parent_id);
                ParentPaginator::new_fetch(
                    (create.parent_id, Default::default()),
                    &auth,
                    size,
                    offset,
                    create.start_from_end(),
                    &self.db,
                )
                .await
            }
            list_by_request::Request::Pager(old) => {
                let pager: ParentPaginator = self.crypto.decode(old.session)?;
                pager.fetch(&auth, size, offset, rev, &self.db).await
            }
        }?;

        let remain = pager.remain(&auth, &self.db).await?;
        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListChatResponse {
            list,
            next_session,
            remain,
        }))
    }
}
