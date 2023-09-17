use crate::{
    common::error::result_into,
    endpoint::{Auth, ControllerTrait},
    init::db::DB,
};
use migration::ValueType;
use sea_orm::EntityTrait;
use tonic::{async_trait, Response};

use super::transform::{Transform, TryTransform};

#[async_trait]
pub trait EditableEndpoint<I>
where
    I: EditableTrait,
    Self: Editable<I> + ControllerTrait,
    <I as EditableTrait>::PrimaryKey :Transform<<I as EditableTrait>::Id>+Send,
    <<<I as EditableTrait>::Entity as sea_orm::EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<<I as EditableTrait>::PrimaryKey>
{
    async fn update<R,T>( 
        &self,
        request: tonic::Request<R>,
    ) -> Result<Response<()>, tonic::Status>
    where
        R: TryTransform<(T,<I as EditableTrait>::PrimaryKey),tonic::Status>+Send,
        T:Send,
    {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let (info, pk) = request.try_into()?;

        let query = Self::rw_filter(I::Entity::find_by_id(pk), auth)?;

        result_into(query.one(db).await)?.ok_or(tonic::Status::not_found("message"))?;

        Ok(Response::new(()))
    }
    async fn create<R,T>(
        &self,
        request: tonic::Request<R>,
    ) -> Result<Response<<I as EditableTrait>::Id>, tonic::Status>
    where
        R: TryTransform<T,tonic::Status>+Send,
        T:Send,
    {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let info = request.try_into()?;

        if Self::can_create(auth){
            let a=result_into(Self::create_model(info).await)?;
            Ok(Response::new(Transform::into(a)))
        }else{
            Err(tonic::Status::permission_denied("message"))
        }
    }
    async fn remove<T>( 
        &self,
        request: tonic::Request<<I as EditableTrait>::Id>,
    ) -> Result<Response<()>, tonic::Status>
    where
        T:Send,
    {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let pk= Transform::into(request);

        let query = Self::rw_filter(I::Entity::delete_by_id(pk), auth)?;

        match result_into(query.exec(db).await)?.rows_affected{
            0 => Err(tonic::Status::not_found("message")),
            _ => Ok(Response::new(())),
        }
    }
}

pub trait EditableTrait {
    type Entity: EntityTrait;
    type PrimaryKey: ValueType;
    type Id: Transform<Self::PrimaryKey> + Send + 'static;
}

#[async_trait]
pub trait Editable<T>
where
    T: EditableTrait,
{
    fn rw_filter<S>(self_: S, auth: Auth) -> Result<S, tonic::Status>;
    fn can_create(auth: Auth) -> bool;
    async fn update_model<R>(
        model: <<T as EditableTrait>::Entity as EntityTrait>::Model,
        info: R,
    ) -> Result<<T as EditableTrait>::PrimaryKey, sea_orm::DbErr>;
    async fn create_model<R>(model: R) -> Result<<T as EditableTrait>::PrimaryKey, sea_orm::DbErr>;
}
