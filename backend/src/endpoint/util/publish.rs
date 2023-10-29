use sea_orm::EntityTrait;
use tonic::{async_trait, Request, Response};

use crate::{common::error::handle_dberr, endpoint::ControllerTrait, init::db::DB};

use super::{intel::IntelTrait, transform::Transform};

pub trait PublishTrait<I: IntelTrait> {
    type Publisher: Send + 'static + Transform<I::PrimaryKey>;
}

#[async_trait]
pub trait Publishable<I: IntelTrait>
where
    I: PublishTrait<I>,
    Self: ControllerTrait,
{
    async fn publish(&self, entity: <I::Entity as EntityTrait>::Model)
        -> Result<(), tonic::Status>;
    async fn unpublish(
        &self,
        entity: <I::Entity as EntityTrait>::Model,
    ) -> Result<(), tonic::Status>;
}

#[async_trait]
pub trait PublishEndpoint<I: PublishTrait<I> + IntelTrait>
where
    Self: Publishable<I> + ControllerTrait,
{
    async fn publish(&self, request: Request<I::Publisher>) -> Result<Response<()>, tonic::Status> {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !perm.can_publish() && !perm.can_root() {
            return Err(tonic::Status::permission_denied("Permission Deny"));
        }

        let pk = Transform::into(request);

        let entity = handle_dberr(I::Entity::find_by_id(pk).one(db).await)?
            .ok_or(tonic::Status::not_found(""))?;

        <Self as Publishable<I>>::publish(&self, entity).await?;

        Ok(Response::new(()))
    }
    async fn unpublish(
        &self,
        request: Request<I::Publisher>,
    ) -> Result<Response<()>, tonic::Status> {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !perm.can_publish() && !perm.can_root() {
            return Err(tonic::Status::permission_denied("Permission Deny"));
        }

        let pk = Transform::into(request);

        let entity = handle_dberr(I::Entity::find_by_id(pk).one(db).await)?
            .ok_or(tonic::Status::not_found(""))?;

        <Self as Publishable<I>>::unpublish(&self, entity).await?;

        Ok(Response::new(()))
    }
}
