use std::time::Duration;

use super::tools::*;

use grpc::backend::token_set_server::*;
use grpc::backend::*;

use crate::entity::token::*;
use crate::entity::*;

const TOKEN_LIMIT: u64 = 16;

impl From<Model> for Token {
    fn from(value: Model) -> Self {
        Token {
            signature: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD_NO_PAD,
                value.rand,
            ),
        }
    }
}

#[async_trait]
impl TokenSet for ArcServer {
    #[instrument(skip_all, level = "debug")]
    async fn list(&self, req: Request<UserId>) -> Result<Response<Tokens>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.ok_or_default()?;

        if req.id != user_id && !perm.root() {
            return Err(Error::Unauthenticated.into());
        }

        let tokens = Entity::find()
            .filter(Column::UserId.eq(user_id))
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
    #[instrument(skip_all, level = "debug")]
    async fn create(&self, req: Request<LoginRequest>) -> Result<Response<TokenInfo>, Status> {
        // FIXME: limit token count
        let (_, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

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
                    .map_err(|err| {
                        trace!("{}", err);
                        Error::BadArgument("expiry")
                    })?;
            let (token, expiry) = self.token.add(&model, dur).in_current_span().await?;

            Ok(Response::new(TokenInfo {
                token: token.into(),
                role: model.permission,
                expiry: into_prost(expiry),
            }))
        } else {
            tracing::trace!("password_mismatch");
            Err(Error::PermissionDeny("wrong password").into())
        }
    }
    #[instrument(skip_all, level = "debug")]
    async fn refresh(
        &self,
        req: Request<prost_wkt_types::Timestamp>,
    ) -> Result<Response<TokenInfo>, Status> {
        let (meta, _, payload) = req.into_parts();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();

            let (user_id, perm) = self.token.verify(token).await?;
            let user = user::Entity::find_by_id(user_id)
                .one(self.db.deref())
                .instrument(info_span!("fetch_user").or_current())
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB)?;

            let time = into_chrono(payload);
            let now = chrono::Utc::now().naive_utc();
            if time < now {
                return Err(Error::BadArgument("").into());
            }

            let dur = time - now;
            self.token
                .remove(token.to_string())
                .in_current_span()
                .await?;

            let (token, expiry) = self.token.add(&user, dur).in_current_span().await?;
            return Ok(Response::new(TokenInfo {
                token: token.into(),
                role: perm as i32,
                expiry: into_prost(expiry),
            }));
        }

        Err(Error::Unauthenticated.into())
    }
    #[instrument(skip_all, level = "debug")]
    async fn logout(&self, req: Request<()>) -> Result<Response<()>, Status> {
        // FIXME: handle rate limiting logic
        let (auth, _) = self.parse_auth(&req).in_current_span().await?;
        auth.ok_or_default()?;

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
