use tonic::{async_trait, Request, Response};

use crate::{common::error::handle_dberr, endpoint::ControllerTrait, init::db::DB};
use sea_orm::*;

use super::{intel::IntelTrait, transform::Transform};

pub trait LinkTrait {
    type Linker: Send
        + 'static
        + Transform<(
            <Self::Intel as IntelTrait>::PrimaryKey,
            <Self::ParentIntel as IntelTrait>::PrimaryKey,
        )>;

    type Intel: IntelTrait;
    type ParentIntel: IntelTrait;
}

#[async_trait]
pub trait Linkable<L>
where
    L: LinkTrait,
{
    async fn link(
        &self,
        model: <<L::Intel as IntelTrait>::Entity as EntityTrait>::Model,
        parent_pk: <L::ParentIntel as IntelTrait>::PrimaryKey,
    ) -> Result<(), tonic::Status>;
    async fn unlink(
        &self,
        model: <<L::Intel as IntelTrait>::Entity as EntityTrait>::Model,
        parent_pk: <L::ParentIntel as IntelTrait>::PrimaryKey,
    ) -> Result<(), tonic::Status>;
}

#[async_trait]
pub trait LinkEndpoint<I>
where
    I: LinkTrait,
    Self: ControllerTrait + Linkable<I>,
{
    async fn link(
        &self,
        request: Request<<I as LinkTrait>::Linker>,
    ) -> Result<Response<()>, tonic::Status> {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !perm.can_link() && !perm.can_root() {
            return Err(tonic::Status::permission_denied("Permission Deny"));
        }

        let (id, ppk) = Transform::into(request);

        let entity = handle_dberr(
            <<I as LinkTrait>::Intel as IntelTrait>::Entity::find_by_id(id)
                .one(db)
                .await,
        )?
        .ok_or(tonic::Status::not_found(""))?;

        <Self as Linkable<I>>::link(&self, entity, ppk).await?;

        Ok(Response::new(()))
    }
    async fn unlink(
        &self,
        request: Request<<I as LinkTrait>::Linker>,
    ) -> Result<Response<()>, tonic::Status> {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !perm.can_link() && !perm.can_root() {
            return Err(tonic::Status::permission_denied("Permission Deny"));
        }

        let (id, ppk) = Transform::into(request);

        let entity = handle_dberr(
            <<I as LinkTrait>::Intel as IntelTrait>::Entity::find_by_id(id)
                .one(db)
                .await,
        )?
        .ok_or(tonic::Status::not_found(""))?;

        <Self as Linkable<I>>::unlink(&self, entity, ppk).await?;

        Ok(Response::new(()))
    }
    async fn full_info_by_parents(
    ) -> Result<<<I as LinkTrait>::Intel as IntelTrait>::FullInfo, tonic::Status> {
        todo!()
    }
}
