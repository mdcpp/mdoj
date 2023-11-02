use tonic::{async_trait, Request, Response};

use crate::{common::error::handle_dberr, endpoint::ControllerTrait, init::db::DB};
use sea_orm::*;

use super::{
    intel::{Intel, IntelTrait},
    stream::TonicStream,
    transform::Transform,
};

pub trait LinkTrait<I: IntelTrait>
where
    <Self::ParentIntel as IntelTrait>::Entity: Related<I::Entity>,
{
    const ENTITY: I::Entity;

    type Linker: Send
        + 'static
        + Transform<(I::PrimaryKey, <Self::ParentIntel as IntelTrait>::PrimaryKey)>;

    type ParentIntel: IntelTrait;
}

#[async_trait]
pub trait Linkable<I: IntelTrait>
where
    I: LinkTrait<I>,
    <I::ParentIntel as IntelTrait>::Entity: Related<I::Entity>,
    Self: ControllerTrait,
{
    async fn link(
        &self,
        model: <I::Entity as EntityTrait>::Model,
        parent_pk: <I::ParentIntel as IntelTrait>::PrimaryKey,
    ) -> Result<(), tonic::Status>;
    async fn unlink(
        &self,
        model: <I::Entity as EntityTrait>::Model,
        parent_pk: <I::ParentIntel as IntelTrait>::PrimaryKey,
    ) -> Result<(), tonic::Status>;
}

#[async_trait]
pub trait LinkQueryable<I: IntelTrait>
where
    I: LinkTrait<I>,
    <I::ParentIntel as IntelTrait>::Entity: Related<I::Entity>,
    Self: ControllerTrait,
{
    async fn owner_filter(
        &self,
        query: Select<<I as IntelTrait>::Entity>,
        owner: <I::ParentIntel as IntelTrait>::PrimaryKey,
    ) -> Result<Select<<I as IntelTrait>::Entity>, tonic::Status>;
}

#[async_trait]
pub trait LinkEndpoint<I: LinkTrait<I> + IntelTrait>
where
    <I::ParentIntel as IntelTrait>::Entity: Related<I::Entity>,
    Self: ControllerTrait,
{
    async fn link(&self, request: Request<I::Linker>) -> Result<Response<()>, tonic::Status>
    where
        Self: Linkable<I>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !perm.can_link() && !perm.can_root() {
            return Err(tonic::Status::permission_denied("Permission Deny"));
        }

        let (id, ppk) = Transform::into(request);

        let entity = handle_dberr(<I as IntelTrait>::Entity::find_by_id(id).one(db).await)?
            .ok_or(tonic::Status::not_found(""))?;

        <Self as Linkable<I>>::link(&self, entity, ppk).await?;

        Ok(Response::new(()))
    }
    async fn unlink(&self, request: Request<I::Linker>) -> Result<Response<()>, tonic::Status>
    where
        Self: Linkable<I>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !perm.can_link() && !perm.can_root() {
            return Err(tonic::Status::permission_denied("Permission Deny"));
        }

        let (id, ppk) = Transform::into(request);

        let entity = handle_dberr(I::Entity::find_by_id(id).one(db).await)?
            .ok_or(tonic::Status::not_found(""))?;

        <Self as Linkable<I>>::unlink(&self, entity, ppk).await?;

        Ok(Response::new(()))
    }
    async fn list_by_parents(
        &self,
        request: tonic::Request<<I::ParentIntel as IntelTrait>::Id>,
    ) -> Result<Response<TonicStream<I::Info>>, tonic::Status>
    where
        Self: LinkQueryable<I>,
        <<I::ParentIntel as IntelTrait>::Entity as EntityTrait>::Model: Sync,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let (_, perm) = auth.ok_or_default()?;

        let ppk = Transform::into(request);

        let parents = handle_dberr(
            <<I::ParentIntel as IntelTrait>::Entity as EntityTrait>::find_by_id(ppk)
                .one(db)
                .await,
        )?
        .ok_or(tonic::Status::not_found(""))?;

        // TODO: admin bypass
        // TODO: use union to expose owner's (private)
        let a = handle_dberr(parents.find_related(I::ENTITY).all(db).await)?;

        todo!()
    }

    // fn full_info_by_parents(
    //     &self,
    //     request: tonic::Request<I::Linker>,
    // ) -> Result<Response<I::FullInfo>, tonic::Status> {
    //     todo!()
    // }
}
