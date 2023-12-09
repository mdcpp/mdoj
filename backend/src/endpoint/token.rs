use std::time::Duration;

use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::token_set_server::*;
use crate::grpc::backend::*;
use crate::grpc::into_prost;

use entity::token::*;
use entity::*;
use tracing::Level;

const TOKEN_LIMIT: u64 = 32;

impl From<String> for Token {
    fn from(value: String) -> Self {
        Token { signature: value }
    }
}

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
impl TokenSet for Arc<Server> {
    #[instrument(skip_all, level = "debug")]
    async fn list(&self, req: Request<UserId>) -> Result<Response<Tokens>, Status> {
        let db = DB.get().unwrap();
        let (auth, _) = self.parse_request(req).await?;
        let (user_id, _) = auth.ok_or_default()?;

        let tokens = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .limit(TOKEN_LIMIT)
            .all(db)
            .await
            .map_err(Into::<Error>::into)?;

        tracing::trace!(token_count = tokens.len(), "retrieve_token");

        Ok(Response::new(Tokens {
            list: tokens.into_iter().map(Into::into).collect(),
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(&self, req: Request<LoginRequest>) -> Result<Response<TokenInfo>, Status> {
        let db = DB.get().unwrap();
        let (_, req) = self.parse_request(req).await?;

        tracing::debug!(username = req.username);

        let model = user::Entity::find()
            .filter(user::Column::Username.eq(req.username))
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("user"))?;

        if self.crypto.hash_eq(req.password.as_str(), &model.password) {
            let dur =
                chrono::Duration::from_std(Duration::from_secs(req.expiry.unwrap_or(60 * 60 * 12)))
                    .map_err(|err| {
                        log::trace!("{}", err);
                        Error::BadArgument("expiry")
                    })?;
            let (token, expiry) = self.token.add(&model, dur).await?;

            Ok(Response::new(TokenInfo {
                token: token.into(),
                permission: model.permission as u64,
                expiry: into_prost(expiry),
            }))
        } else {
            tracing::trace!("password_mismatch");
            Err(Error::PremissionDeny("password").into())
        }
    }
    #[instrument(skip_all, level = "debug")]
    async fn refresh(&self, req: Request<()>) -> Result<Response<TokenInfo>, Status> {
        let db = DB.get().unwrap();
        let (meta, _, _) = req.into_parts();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();

            let (user_id, perm) = self.token.verify(token).await?;
            let user = user::Entity::find_by_id(user_id)
                .one(db)
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB("user"))?;

            let dur = chrono::Duration::from_std(Duration::from_secs(60 * 60 * 12)).unwrap();
            self.token.remove(token.to_string()).await?;

            let (token, expiry) = self.token.add(&user, dur).await?;
            return Ok(Response::new(TokenInfo {
                token: token.into(),
                permission: perm.0 as u64,
                expiry: into_prost(expiry),
            }));
        }

        Err(Error::Unauthenticated.into())
    }
    #[instrument(skip_all, level = "debug")]
    async fn logout(&self, req: Request<()>) -> Result<Response<()>, Status> {
        let meta = req.metadata();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();

            self.token.remove(token.to_string()).await?;
            tracing::event!(Level::TRACE, token = token);

            return Ok(Response::new(()));
        }

        Err(Error::Unauthenticated.into())
    }
}
