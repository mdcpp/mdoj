use std::pin::Pin;

use crate::{
    common::error::result_into, endpoint::*, grpc::proto::prelude::*, impl_id, init::db::DB, Server,
};

use super::util::{intel::*, transform::Transform};
use futures::Future;
use tonic::{Request, Response};

use entity::problem::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Select, QuerySelect, PaginatorTrait,
};

macro_rules! insert_if_exists {
    ($target:expr,$src:expr , $field:ident) => {
        if let Some(x) = $src.$field {
            $target.$field = ActiveValue::Set(x);
        }
    };
    ($target:expr,$src:expr, $field:ident, $($ext:ident),+) => {
        insert_if_exists!($target,$src, $field);
        insert_if_exists!($target,$src, $($ext),+);
    };
}

fn vr_to_rv<T, E>(v: Vec<Result<T, E>>) -> Result<Vec<T>, E> {
    v.into_iter().collect()
}
fn vo_to_ov<T>(v: Vec<Option<T>>) -> Option<Vec<T>> {
    v.into_iter().collect()
}

pub struct ProblemIntel;

impl IntelTrait for ProblemIntel {
    type Entity = Entity;

    type PartialModel = PartialProblem;

    type InfoArray = Problems;

    type FullInfo = ProblemFullInfo;

    type Info = ProblemInfo;

    type PrimaryKey = i32;

    type Id = ProblemId;

    type UpdateInfo = update_problem_request::Info;

    type CreateInfo = create_problem_request::Info;
}

#[async_trait]
impl Intel<ProblemIntel> for Server {
    fn ro_filter<S>(query: S, auth: Auth) -> Result<S, tonic::Status>
    where
        S: QueryFilter,
    {
        Ok(match auth {
            Auth::Guest => query.filter(Column::Public.eq(true)),
            Auth::User((user_id, perm)) => match perm.can_root() || perm.can_manage_problem() {
                true => query,
                false => query.filter(Column::Public.eq(true).or(Column::UserId.eq(user_id))),
            },
        })
    }

    fn rw_filter<S>(query: S, auth: Auth) -> Result<S, tonic::Status>
    where
        S: QueryFilter,
    {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() {
            Ok(query)
        } else if perm.can_manage_problem() {
            Ok(query.filter(Column::UserId.eq(user_id)))
        } else {
            Err(tonic::Status::permission_denied("User cannot write"))
        }
    }

    fn can_create(auth: Auth) -> Result<(), tonic::Status> {
        let (_, perm) = auth.ok_or_default()?;
        match perm.can_root() || perm.can_manage_problem() {
            true => Ok(()),
            false => Err(tonic::Status::unauthenticated("Permission Deny")),
        }
    }

    async fn update_model(
        model: Model,
        info: update_problem_request::Info,
    ) -> Result<i32, tonic::Status> {
        let db = DB.get().unwrap();
        let user_id = model.user_id;

        let mut target = model.into_active_model();
        insert_if_exists!(target, info, title, content, memory, time, difficulty, tags);

        if let Some(tests) = info.tests {
            let list = tests.list.clone();

            let futs: Vec<_> = list
                .into_iter()
                .map(|testcase_id| {
                    entity::testcase::Entity::find_by_id(testcase_id.id)
                        .filter(entity::testcase::Column::UserId.eq(user_id))
                        .count(db)
                })
                .into_iter()
                .collect();

            let list = result_into(vr_to_rv(futures::future::join_all(futs).await))?;

            let vaild=list.into_iter().map(|x| x==0).reduce(|x,y|x||y).unwrap_or(true);
            
            if vaild{
                todo!("Update testcases's problem_id")
            }else{
                return Err(tonic::Status::permission_denied(
                    "adding unowned testcase, consider take ownership of the testcases",
                ));
            };
        };

        todo!("commit changes(active model)")
    }

    async fn create_model(model: create_problem_request::Info) -> Result<i32, tonic::Status> {
        todo!()
    }
}

// impl IntelEndpoint<ProblemIntel> for Server {}

impl Transform<ProblemId> for i32 {
    fn into(self) -> ProblemId {
        todo!()
    }
}

impl Transform<<Entity as EntityTrait>::Column> for SortBy {
    fn into(self) -> <<ProblemIntel as IntelTrait>::Entity as EntityTrait>::Column {
        match self {
            SortBy::SubmitCount => Column::SubmitCount,
            SortBy::AcRate => Column::AcRate,
            SortBy::Difficulty => Column::Difficulty,
            _ => Column::Id,
        }
    }
}

impl Transform<Problems> for Vec<ProblemInfo> {
    fn into(self) -> Problems {
        let list = self
            .into_iter()
            .map(|x| ProblemInfo {
                id: x.id,
                title: x.title,
                submit_count: x.submit_count,
                ac_rate: x.ac_rate,
            })
            .collect();
        Problems { list }
    }
}

impl Transform<<ProblemIntel as IntelTrait>::Info> for PartialProblem {
    fn into(self) -> <ProblemIntel as IntelTrait>::Info {
        ProblemInfo {
            id: Some(ProblemId { id: self.id }),
            title: self.title,
            submit_count: self.submit_count,
            ac_rate: self.ac_rate,
        }
    }
}

impl Transform<ProblemFullInfo> for Model {
    fn into(self) -> ProblemFullInfo {
        todo!()
    }
}

impl_id!(Problem);

macro_rules! insert_if_exists {
    ($model:ident, $value:expr, $field:ident) => {
        if let Some(x) = $value.$field {
            $model.$field = ActiveValue::Set(x);
        }
    };
    ($model:ident, $value:expr, $field:ident, $($ext:ident),+) => {
        insert_if_exists!($model, $value, $field);
        insert_if_exists!($model, $value, $($ext),+);
    };
}

impl BaseEndpoint<ProblemIntel> for Server {}

#[async_trait]
impl problem_set_server::ProblemSet for Server {
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        BaseEndpoint::list(self, request).await
    }

    async fn search_by_text(
        &self,
        request: Request<TextSearchRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        BaseEndpoint::search_by_text(self, request, &[Column::Title, Column::Content]).await
    }

    async fn search_by_tag(
        &self,
        request: Request<TextSearchRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        BaseEndpoint::search_by_text(self, request, &[Column::Tags]).await
    }

    async fn full_info(
        &self,
        request: Request<ProblemId>,
    ) -> Result<Response<ProblemFullInfo>, tonic::Status> {
        BaseEndpoint::full_info(self, request).await
    }

    async fn create(
        &self,
        request: tonic::Request<CreateProblemRequest>,
    ) -> Result<Response<ProblemId>, tonic::Status> {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let info = request
            .info
            .ok_or(tonic::Status::invalid_argument("No info"))?;

        match auth {
            Auth::Guest => Err(tonic::Status::permission_denied("Guest cannot create")),
            Auth::User((user_id, perm)) => {
                if perm.can_root() || perm.can_manage_problem() {
                    let db_result = ActiveModel {
                        user_id: ActiveValue::Set(user_id),
                        success: ActiveValue::Set(0),
                        submit_count: ActiveValue::Set(0),
                        ac_rate: ActiveValue::Set(1.0),
                        memory: ActiveValue::Set(info.memory),
                        time: ActiveValue::Set(info.time),
                        difficulty: ActiveValue::Set(info.difficulty),
                        public: ActiveValue::Set(false),
                        tags: ActiveValue::Set("".to_string()),
                        title: ActiveValue::Set(info.title),
                        content: ActiveValue::Set(info.content),
                        ..Default::default()
                    }
                    .insert(db)
                    .await; // TODO: testcase
                    let id = result_into(db_result)?.id;
                    Ok(Response::new(ProblemId { id }))
                } else {
                    Err(tonic::Status::permission_denied("User cannot create"))
                }
            }
        }
    }

    async fn update(
        &self,
        request: tonic::Request<UpdateProblemRequest>,
    ) -> Result<Response<()>, tonic::Status> {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let info = request
            .info
            .ok_or(tonic::Status::invalid_argument("No info"))?;

        let pk = request
            .id
            .ok_or(tonic::Status::invalid_argument("No id"))?
            .id;

        match auth {
            Auth::Guest => Err(tonic::Status::permission_denied("Guest cannot create")),
            Auth::User((user_id, perm)) => match perm.can_root() || perm.can_manage_problem() {
                true => {
                    let mut tar = result_into(Entity::find_by_id(pk).one(db).await)?
                        .ok_or(tonic::Status::not_found("message"))?
                        .into_active_model();
                    insert_if_exists!(tar, info, title, content, memory, time, difficulty, tags);
                    // TODO: only root can manage other's problem
                    Ok(Response::new(()))
                }
                false => Err(tonic::Status::permission_denied("User cannot create")),
            },
        }
    }

    async fn remove(
        &self,
        request: tonic::Request<ProblemId>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn link(
        &self,
        request: tonic::Request<ProblemLink>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn unlink(
        &self,
        request: tonic::Request<ProblemLink>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn add_test(
        &self,
        request: tonic::Request<Testcase>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn remove_test(
        &self,
        request: tonic::Request<TestcaseId>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    #[doc = " Server streaming response type for the Rejudge method."]
    type RejudgeStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<(), tonic::Status>> + Send>>;

    async fn rejudge(
        &self,
        request: tonic::Request<ProblemId>,
    ) -> Result<Response<Self::RejudgeStream>, tonic::Status> {
        todo!()
    }
}
