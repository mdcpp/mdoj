use super::tools::*;

use grpc::backend::chat_server::*;

use crate::entity::chat::{Paginator, *};

impl<'a> From<WithAuth<'a, Model>> for ChatInfo {
    fn from(value: WithAuth<'a, Model>) -> Self {
        let model = value.1;
        let writable = Entity::writable(&model, value.0);
        ChatInfo {
            id: model.id,
            user_id: model.user_id,
            problem_id: model.problem_id,
            create_at: into_prost(model.create_at),
            message: model.message,
            writable,
        }
    }
}

impl<'a> WithAuthTrait for Model {}

#[tonic::async_trait]
impl Chat for ArcServer {
    async fn create(&self, req: Request<CreateChatRequest>) -> Result<Response<Id>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, _) = auth.auth_or_guest()?;

        req.bound_check()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<Id>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(model, req, message);

        let model = model
            .save(self.db.deref())
            .instrument(info_span!("save").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        let id: Id = model.id.clone().unwrap().into();
        self.dup.store(user_id, uuid, id.clone());

        tracing::debug!(id = id.id, "chat_created");

        Ok(Response::new(id))
    }

    async fn remove(&self, req: Request<Id>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let result = Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(self.db.deref())
            .instrument(info_span!("remove").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        if result.rows_affected == 0 {
            return Err(Error::NotInDB.into());
        }

        tracing::debug!(id = req.id, "chat_remove");

        Ok(Response::new(()))
    }

    async fn list(
        &self,
        req: Request<ListChatRequest>,
    ) -> Result<Response<ListChatResponse>, Status> {
        let (auth, req) = self
            .parse_request_fn(req, |req| {
                (req.size + req.offset.saturating_abs() as u64 / 5 + 2)
                    .try_into()
                    .unwrap_or(u32::MAX)
            })
            .await?;

        req.bound_check()?;

        let paginator = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_chat_request::Request::Create(create) => {
                let start_from_end = create.order == Order::Descend as i32;
                Paginator::new(create.problem_id, start_from_end)
            }
            list_chat_request::Request::Paginator(x) => self.crypto.decode(x)?,
        };
        let mut paginator = paginator.with_auth(&auth).with_db(&self.db);

        let list = paginator.fetch(req.size, req.offset).await?;
        let remain = paginator.remain().await?;

        let paginator = paginator.into_inner();

        Ok(Response::new(ListChatResponse {
            list: list
                .into_iter()
                .map(|x| x.with_auth(&auth).into())
                .collect(),
            paginator: self.crypto.encode(paginator)?,
            remain,
        }))
    }
}
