use std::num::NonZeroU32;
use std::time::Duration;

use super::tools::*;

use grpc::backend::token_server::*;

use crate::entity::token::*;
use crate::entity::*;
use crate::util::rate_limit::RateLimit;

const TOKEN_LIMIT: u64 = 16;

impl From<Model> for String {
    fn from(value: Model) -> Self {
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            value.rand,
        )
    }
}

#[async_trait]
impl Token for ArcServer {
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Token.list",
        err(level = "debug", Display)
    )]
    async fn list(&self, req: Request<Id>) -> Result<Response<Tokens>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        let (user_id, perm) = auth.assume_login()?;

        if req.id != user_id {
            perm.admin()?;
        }

        let tokens = Entity::find()
            .filter(Column::Id.eq(user_id))
            .limit(TOKEN_LIMIT)
            .all(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        tracing::trace!(token_count = tokens.len(), "retrieve_token");

        Ok(Response::new(Tokens {
            list: tokens.into_iter().map(Into::into).collect(),
        }))
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Token.create",
        err(level = "debug", Display)
    )]
    async fn create(&self, req: Request<LoginRequest>) -> Result<Response<TokenInfo>, Status> {
        // FIXME: add request_id
        let (_, req) = self.rate_limit(req).in_current_span().await?;

        tracing::debug!(username = req.username);

        let model = user::Entity::find()
            .filter(user::Column::Username.eq(req.username))
            .one(self.db.deref())
            .instrument(info_span!("fetch_user").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        if self.crypto.hash_eq(req.password.as_str(), &model.password) {
            let dur =
                chrono::Duration::from_std(Duration::from_secs(req.expiry.unwrap_or(60 * 60 * 12)))
                    .map_err(|err| Error::BadArgument("expiry"))?;
            let (token, expiry) = self.token.add(&model, dur).in_current_span().await?;

            Ok(Response::new(TokenInfo {
                token,
                role: model.permission,
                expiry: into_prost(expiry),
            }))
        } else {
            tracing::trace!("password_mismatch");
            Err(Error::PermissionDeny("wrong password").into())
        }
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Token.refresh",
        err(level = "debug", Display)
    )]
    async fn refresh(&self, req: Request<RefreshRequest>) -> Result<Response<TokenInfo>, Status> {
        let (auth, bucket) = self.parse_auth(&req).in_current_span().await?;
        let (meta, _, req) = req.into_parts();
        bucket.cost(NonZeroU32::new(req.get_cost()).unwrap())?;

        if let Some(x) = meta.get("token") {
            req.get_or_insert(|req| async move {
                let token = x.to_str().unwrap();

                let (user_id, perm) = self.token.verify(token).await?;
                let user = user::Entity::find_by_id(user_id)
                    .one(self.db.deref())
                    .instrument(info_span!("fetch_user").or_current())
                    .await
                    .map_err(Into::<Error>::into)?
                    .ok_or(Error::NotInDB)?;

                let dur = chrono::Duration::from_std(Duration::from_secs(
                    req.expiry.unwrap_or(60 * 60 * 12),
                ))
                .map_err(|err| Error::BadArgument("expiry"))?;

                self.token.remove(token.to_string()).await?;
                let (token, expiry) = self.token.add(&user, dur).in_current_span().await?;
                Ok(TokenInfo {
                    token,
                    role: perm as i32,
                    expiry: into_prost(expiry),
                })
            })
            .await
            .with_grpc()
            .into()
        } else {
            Err(Error::Unauthenticated.into())
        }
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Token.logouy",
        err(level = "debug", Display)
    )]
    async fn logout(&self, req: Request<()>) -> Result<Response<()>, Status> {
        let (auth, bucket) = self.parse_auth(&req).in_current_span().await?;
        auth.assume_login()?;
        bucket.cost(NonZeroU32::new(10).unwrap())?;

        if let Some(x) = req.metadata().get("token") {
            let token = x.to_str().unwrap();

            self.token
                .remove(token.to_string())
                .in_current_span()
                .await?;
            tracing::event!(Level::TRACE, token = token);

            return Ok(Response::new(()));
        }

        Err(Error::Unauthenticated.into())
    }
}
