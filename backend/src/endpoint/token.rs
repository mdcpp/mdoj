use std::time::Duration;

use super::endpoints::*;
use super::tools::*;
use super::util::time::into_prost;

use crate::endpoint::util::hash::hash_eq;
use crate::grpc::backend::token_set_server::*;
use crate::grpc::backend::*;

use entity::token::*;
use entity::*;

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
    async fn list(&self, req: Request<UserId>) -> Result<Response<Tokens>, Status> {
        let db = DB.get().unwrap();
        let (auth, _) = self.parse_request(req).await?;
        let (user_id, _) = auth.ok_or_default()?;

        let tokens = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await
            .map_err(Into::<Error>::into)?;

        Ok(Response::new(Tokens {
            list: tokens.into_iter().map(Into::into).collect(),
        }))
    }
    async fn create(&self, req: Request<LoginRequest>) -> Result<Response<TokenInfo>, Status> {
        let db = DB.get().unwrap();
        let (_, req) = self.parse_request(req).await?;

        let model = user::Entity::find()
            .filter(user::Column::Username.eq(req.username))
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("user"))?;

        if hash_eq(req.password.as_str(), &model.password) {
            let dur =
                chrono::Duration::from_std(Duration::from_secs(req.expiry.unwrap_or(60 * 60 * 12)))
                    .map_err(|err| {
                        log::trace!("{}", err);
                        Error::BadArgument("expiry")
                    })?;
            let (token, expiry) = self
                .token
                .add(&model, dur)
                .await
                .map_err(Into::<Error>::into)?;

            Ok(Response::new(TokenInfo {
                token: token.into(),
                permission: model.permission,
                expiry: into_prost(expiry),
            }))
        } else {
            Err(Error::PremissionDeny("password").into())
        }
    }
    async fn refresh(&self, req: Request<()>) -> Result<Response<TokenInfo>, Status> {
        let db = DB.get().unwrap();
        let (meta, _, _) = req.into_parts();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();
            let pack = self
                .token
                .verify(token)
                .await
                .map_err(Into::<Error>::into)?;

            let (user_id, perm) = pack.ok_or(Error::Unauthenticated)?;
            let user = user::Entity::find_by_id(user_id)
                .one(db)
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB("user"))?;

            let dur = chrono::Duration::from_std(Duration::from_secs(60 * 60 * 12)).unwrap();
            self.token
                .remove(token.to_string())
                .await
                .map_err(Into::<Error>::into)?;

            let (token, expiry) = self
                .token
                .add(&user, dur)
                .await
                .map_err(Into::<Error>::into)?;
            return Ok(Response::new(TokenInfo {
                token: token.into(),
                permission: perm.0,
                expiry: into_prost(expiry),
            }));
        }

        Err(Error::Unauthenticated.into())
    }
    async fn logout(&self, req: Request<()>) -> Result<Response<()>, Status> {
        let meta = req.metadata();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();

            self.token
                .remove(token.to_string())
                .await
                .map_err(Into::<Error>::into)?;
        }

        Err(Error::Unauthenticated.into())
    }
}
