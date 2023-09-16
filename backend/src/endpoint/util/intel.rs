use sea_orm::*;
use tonic::{async_trait, Request, Response};

use crate::common::error::result_into;
use crate::endpoint::*;
use crate::grpc::proto::prelude::{ListRequest, Page, SearchByTextRequest, SortBy};
use crate::init::db::DB;

pub trait Transform<I> {
    fn into(self) -> I;
}

#[async_trait]
pub trait Endpoint<I>
where
    I: IntelTrait,
    Self: Intel<I> + ControllerTrait,
    <I as IntelTrait>::PrimaryKey: Transform<
            <<<I as IntelTrait>::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType,
        > + Send,
{
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<<I as IntelTrait>::InfoArray>, tonic::Status>
    where
        SortBy: Transform<<<I as IntelTrait>::Entity as EntityTrait>::Column>,
        Vec<<I as IntelTrait>::Info>: Transform<<I as IntelTrait>::InfoArray>,
        <I as IntelTrait>::PartialModel: Transform<<I as IntelTrait>::Info>+ Send,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let query = I::Entity::find()
            .select_only()
            .columns(I::INFO_INTERESTS.iter().cloned());
        let query = Self::ro_filter(query, auth)?;

        let sort_by = SortBy::from_i32(request.sort_by)
            .ok_or(tonic::Status::invalid_argument("Invalid sort_by"))?;
        let page = request
            .page
            .ok_or(tonic::Status::invalid_argument("Invalid page"))?;
        let reverse = request.reverse;

        let query = Self::sort_filter(query, sort_by, page, reverse);

        let list: Vec<<I as IntelTrait>::PartialModel> = result_into(query.into_partial_model().all(db).await)?;
        let list: Vec<<I as IntelTrait>::Info> = list.into_iter().map(|x| Transform::into(x)).collect();
        Ok(Response::new(Transform::into(list)))
    }
    async fn search_by_text(
        &self,
        request: Request<SearchByTextRequest>,
        text: &'static [<<I as IntelTrait>::Entity as EntityTrait>::Column],
    ) -> Result<Response<<I as IntelTrait>::InfoArray>, tonic::Status>
    where
        SortBy: Transform<<<I as IntelTrait>::Entity as EntityTrait>::Column>,
        <I as IntelTrait>::InfoArray: From<Vec<<I as IntelTrait>::Info>>,
        <I as IntelTrait>::PartialModel: Transform<<I as IntelTrait>::Info>+ Send,
    {
        debug_assert!(text.len() > 0);
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let query = I::Entity::find()
            .select_only()
            .columns(I::INFO_INTERESTS.iter().cloned());
        let query = Self::ro_filter(query, auth)?;

        let sort_by = SortBy::from_i32(request.sort_by)
            .ok_or(tonic::Status::invalid_argument("Invalid sort_by"))?;
        let page = request
            .page
            .ok_or(tonic::Status::invalid_argument("Invalid page"))?;
        let reverse = false;

        let query = Self::sort_filter(query, sort_by, page, reverse);

        let mut condition = text[0].like(request.text.clone());
        for col in text[1..].iter() {
            condition = condition.or(col.like(request.text.clone()));
        }

        let query = query.filter(condition);

        
        let list: Vec<<I as IntelTrait>::PartialModel> = result_into(query.into_partial_model().all(db).await)?;
        let list: Vec<<I as IntelTrait>::Info> = list.into_iter().map(|x| Transform::into(x)).collect();

        Ok(Response::new(list.into()))
    }
    async fn full_info<Id>(
        &self,
        request: Request<Id>,
    ) -> Result<Response<<I as IntelTrait>::FullInfo>, tonic::Status>
    where
        <<I as IntelTrait>::Entity as EntityTrait>::Model: Transform<<I as IntelTrait>::FullInfo>,
        Id: Transform<<I as IntelTrait>::PrimaryKey> + Send,
        <<<I as IntelTrait>::Entity as sea_orm::EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<<I as IntelTrait>::PrimaryKey>
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let pk:<I as IntelTrait>::PrimaryKey = Transform::into(request);
        let query = I::Entity::find_by_id(pk);
        let query = Self::ro_filter(query, auth)?;

        let info: <I as IntelTrait>::FullInfo = Transform::into(
            result_into(query.one(db).await)?.ok_or(tonic::Status::not_found("Not found"))?,
        );
        Ok(Response::new(info))
    }
}

pub trait IntelTrait {
    type Entity: EntityTrait;

    type PartialModel: PartialModelTrait;

    type InfoArray;
    type FullInfo;
    type Info;
    type PrimaryKey;
    const INFO_INTERESTS: &'static [<<Self as IntelTrait>::Entity as EntityTrait>::Column];
}

pub trait Intel<T>
where
    T: IntelTrait,
{
    fn ro_filter(self_: Select<T::Entity>, auth: Auth) -> Result<Select<T::Entity>, tonic::Status>;
    fn sort_filter(
        self_: Select<T::Entity>,
        sort_by: SortBy,
        page: Page,
        reverse: bool,
    ) -> Select<T::Entity>
    where
        SortBy: Transform<<<T as IntelTrait>::Entity as EntityTrait>::Column>,
    {
        let col = Transform::into(sort_by);

        let self_ = if reverse {
            self_.order_by_asc(col)
        } else {
            self_.order_by_desc(col)
        };

        let offset = page.offset as u64;
        let limit = page.amount as u64;
        self_.clone().offset(offset).limit(limit)
    }
}
