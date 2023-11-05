use tonic::{async_trait, Request, Response};

use super::super::tools::*;
use sea_orm::*;

use super::{
    intel::{Intel, IntelTrait},
    stream::{into_tokiostream, TonicStream},
    transform::Transform,
};

pub trait LinkTrait<I: IntelTrait>
where
    <Self::ParentIntel as IntelTrait>::Entity: Related<I::Entity>,
{
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
    ) -> Result<(), Error>;
    async fn unlink(
        &self,
        model: <I::Entity as EntityTrait>::Model,
        parent_pk: <I::ParentIntel as IntelTrait>::PrimaryKey,
    ) -> Result<(), Error>;
}

#[async_trait]
pub trait LinkQueryable<I: IntelTrait>
where
    I: LinkTrait<I>,
    <I::ParentIntel as IntelTrait>::Entity: Related<I::Entity>,
    Self: ControllerTrait,
{
    async fn ro_filter(
        query: Select<<I as IntelTrait>::Entity>,
        auth: Auth,
        parent_pk: <I::ParentIntel as IntelTrait>::PrimaryKey,
    ) -> Result<Select<<I as IntelTrait>::Entity>, Error>;
}

#[async_trait]
pub trait LinkEndpoint<I: LinkTrait<I> + IntelTrait>
where
    <I::ParentIntel as IntelTrait>::Entity: Related<I::Entity>,
    Self: ControllerTrait + Intel<I>,
{
    async fn link(&self, request: Request<I::Linker>) -> Result<Response<()>, Error>
    where
        Self: Linkable<I>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !perm.can_link() && !perm.can_root() {
            return Err(Error::PremissionDeny(
                "Only User with `can_link` can associate two entity",
            ));
        }

        let (id, ppk) = Transform::into(request);

        let entity = <I as IntelTrait>::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(Error::NotInDB(I::NAME))?;

        <Self as Linkable<I>>::link(&self, entity, ppk).await?;

        Ok(Response::new(()))
    }
    async fn unlink(&self, request: Request<I::Linker>) -> Result<Response<()>, Error>
    where
        Self: Linkable<I>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !perm.can_link() && !perm.can_root() {
            return Err(Error::PremissionDeny(
                "Only User with `can_link` can associate two entity",
            ));
        }

        let (id, ppk) = Transform::into(request);

        let entity = I::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(Error::NotInDB(I::NAME))?;

        <Self as Linkable<I>>::unlink(&self, entity, ppk).await?;

        Ok(Response::new(()))
    }
    async fn list_by_parents(
        &self,
        request: tonic::Request<<I::ParentIntel as IntelTrait>::Id>,
    ) -> Result<Response<TonicStream<I::Info>>, Error>
    where
        Self: LinkQueryable<I>,
        <I::Entity as EntityTrait>::Model: Sync,
        I::PartialModel: Transform<<I as IntelTrait>::Info>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;
        let (_, perm) = auth.ok_or_default()?;
        let ppk = Transform::into(request);

        let parents = <<I::ParentIntel as IntelTrait>::Entity as EntityTrait>::find_by_id(ppk)
            .one(db)
            .await?
            .ok_or(Error::NotInDB(I::NAME))?;

        let mut query = parents.find_related(I::Entity::default());
        if !perm.can_root() {
            query = <Self as LinkQueryable<I>>::ro_filter(query, auth, ppk).await?;
        }

        let list: Vec<I::PartialModel> = query.into_partial_model().all(db).await?;
        let output_stream = into_tokiostream(list.into_iter().map(|x| Transform::into(x)));

        Ok(Response::new(Box::pin(output_stream)))
    }

    async fn full_info_by_parents(
        &self,
        request: tonic::Request<I::Linker>,
    ) -> Result<Response<I::FullInfo>, Error>
    where
        Self: LinkQueryable<I>,
        <I::Entity as EntityTrait>::Model: Sync + Transform<<I as IntelTrait>::FullInfo>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;
        let (_, perm) = auth.ok_or_default()?;
        let (id, ppk) = Transform::into(request);

        let mut query = I::Entity::find_by_id(id);
        if !perm.can_root() {
            query = <Self as LinkQueryable<I>>::ro_filter(query, auth, ppk).await?;
        }

        let model = query.one(db).await?.ok_or(Error::NotInPayload(""))?;

        Ok(Response::new(Transform::into(model)))
    }
}
