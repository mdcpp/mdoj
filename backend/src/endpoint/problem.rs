use std::pin::Pin;

use crate::{
    common::error::handle_dberr,
    endpoint::*,
    grpc::proto::prelude::*,
    impl_id,
    init::db::{self, DB},
    parse_option, Server,
};

use super::util::{intel::*, link::*, publish::*, transform::*};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{Request, Response};

use entity::{education, problem::*, testcase::Tests};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait,
    PaginatorTrait, QueryFilter, QuerySelect,
};

type TonicStream<T> = Pin<Box<dyn tokio_stream::Stream<Item = Result<T, tonic::Status>> + Send>>;

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

// setup ZST(ProblemIntel for Problem)
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

    fn can_create(auth: Auth) -> Result<i32, tonic::Status> {
        let (user_id, perm) = auth.ok_or_default()?;
        match perm.can_root() || perm.can_manage_problem() {
            true => Ok(user_id),
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

            let list = handle_dberr(vr_to_rv(futures::future::join_all(futs).await))?;

            let vaild = list
                .into_iter()
                .map(|x| x == 0)
                .reduce(|x, y| x || y)
                .unwrap_or(true);

            if vaild {
                todo!("Update testcases's problem_id")
            } else {
                return Err(tonic::Status::permission_denied(
                    "adding unowned testcase, consider take ownership of the testcases",
                ));
            };
        };

        todo!("commit changes(active model)")
    }

    async fn create_model(
        model: create_problem_request::Info,
        user_id: i32,
    ) -> Result<i32, tonic::Status> {
        let db = DB.get().unwrap();

        let model = handle_dberr(
            ActiveModel {
                user_id: ActiveValue::Set(user_id),
                ac_rate: ActiveValue::Set(0.0),
                memory: ActiveValue::Set(model.memory),
                time: ActiveValue::Set(model.time),
                difficulty: ActiveValue::Set(model.difficulty),
                tags: ActiveValue::Set(model.tags),
                title: ActiveValue::Set(model.title),
                content: ActiveValue::Set(model.content),
                ..Default::default()
            }
            .insert(db)
            .await,
        )?;
        Ok(model.id)
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

// TODO, use specialized impl under ``Intel`` for such (operation?)
#[async_trait]
impl AsyncTransform<Result<ProblemFullInfo, tonic::Status>> for Model {
    async fn into(self) -> Result<ProblemFullInfo, tonic::Status> {
        let db = DB.get().unwrap();

        let edu = handle_dberr(
            self.find_related(education::Entity)
                .select_only()
                .columns([education::Column::Id])
                .one(db)
                .await,
        )?;
        let education_id = edu.map(|x| EducationId { id: x.id });

        Ok(ProblemFullInfo {
            content: self.content,
            memory: self.memory,
            time: self.time,
            difficulty: self.difficulty,
            tags: self.tags,
            public: self.public,
            info: Some(ProblemInfo {
                id: Some(ProblemId { id: self.id }),
                title: self.title,
                submit_count: self.submit_count,
                ac_rate: self.ac_rate,
            }),
            education_id,
        })
    }
}

impl_id!(Problem);

impl TryTransform<create_problem_request::Info, tonic::Status> for CreateProblemRequest {
    fn try_into(self) -> Result<create_problem_request::Info, tonic::Status> {
        let info = self
            .info
            .ok_or(tonic::Status::invalid_argument("info not found"))?;
        Ok(info)
    }
}

impl TryTransform<(update_problem_request::Info, i32), tonic::Status> for UpdateProblemRequest {
    fn try_into(self) -> Result<(update_problem_request::Info, i32), tonic::Status> {
        let info = self
            .info
            .ok_or(tonic::Status::invalid_argument("info not found"))?;
        let id = self
            .id
            .map(|x| x.id)
            .ok_or(tonic::Status::invalid_argument("id not found"))?;
        Ok((info, id))
    }
}

// pub struct ProblemLink;

// impl LinkTrait for ProblemLink {
//     type Linker = ProblemLink;
//     type Intel = ProblemIntel;

//     type ParentIntel;
// }

struct ProblemPublish;

impl PublishTrait for ProblemPublish {
    type Publisher = ProblemId;
    type Intel = ProblemIntel;
}

#[async_trait]
impl Publishable<ProblemPublish> for Server {
    async fn publish(&self, entity: Model) -> Result<(), tonic::Status> {
        let db = DB.get().unwrap();

        let mut model = entity.into_active_model();
        model.public = ActiveValue::Set(true);

        handle_dberr(model.save(db).await)?;

        Ok(())
    }

    async fn unpublish(&self, entity: Model) -> Result<(), tonic::Status> {
        let db = DB.get().unwrap();

        let mut model = entity.into_active_model();
        model.public = ActiveValue::Set(false);

        handle_dberr(model.save(db).await)?;

        Ok(())
    }
}

// impl Endpoints
impl BaseEndpoint<ProblemIntel> for Server {}
impl PublishEndpoint<ProblemPublish> for Server {}

// Adapters
#[async_trait]
impl problem_set_server::ProblemSet for Server {
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
        BaseEndpoint::create(self, request).await
    }

    async fn update(
        &self,
        request: tonic::Request<UpdateProblemRequest>,
    ) -> Result<Response<()>, tonic::Status> {
        BaseEndpoint::update(self, request).await
    }

    async fn remove(
        &self,
        request: tonic::Request<ProblemId>,
    ) -> Result<Response<()>, tonic::Status> {
        BaseEndpoint::remove(self, request).await
    }

    async fn link(
        &self,
        request: tonic::Request<ProblemLink>,
    ) -> Result<Response<()>, tonic::Status> {
        let db = DB.get().unwrap();

        let (auth, payload) = self.parse_request(request).await?;

        let contest_id = parse_option!(payload, contest_id).id;
        let problem_id = parse_option!(payload, problem_id).id;

        let (_, perm) = auth.ok_or_default()?;

        if perm.can_link() {
            let mut problem = handle_dberr(
                Entity::find_by_id(problem_id)
                    .select_only()
                    .columns([Column::ContestId])
                    .one(db)
                    .await,
            )?
            .ok_or(tonic::Status::not_found("problem not found"))?
            .into_active_model();
            problem.contest_id = ActiveValue::Set(contest_id);
            handle_dberr(problem.update(db).await).map(|_| Response::new(()))
        } else {
            Err(tonic::Status::permission_denied("User cannot link"))
        }
    }

    async fn unlink(
        &self,
        request: tonic::Request<ProblemLink>,
    ) -> Result<Response<()>, tonic::Status> {
        // let (auth, payload) = self.parse_request(request).await?;

        // let (_, perm) = auth.ok_or_default()?;

        // if perm.can_root() || perm.can_link() {
        //     let db = DB.get().unwrap();
        //     let contest_id = payload
        //         .contest_id
        //         .ok_or(tonic::Status::not_found("contest id not found"))?
        //         .id;
        //     let problem_id = payload
        //         .problem_id
        //         .ok_or(tonic::Status::not_found("problem id not found"))?
        //         .id;
        //     let mut problem = handle_dberr(
        //         Entity::find_by_id(problem_id)
        //             .select_only()
        //             .columns([Column::ContestId])
        //             .one(db)
        //             .await,
        //     )?
        //     .ok_or(tonic::Status::not_found("problem not found"))?
        //     .into_active_model();
        //     problem.contest_id = ActiveValue::Set(contest_id);
        //     handle_dberr(problem.update(db).await).map(|_| Response::new(()))
        // } else {
        //     Err(tonic::Status::permission_denied(""))
        // }
        todo!()
    }

    async fn publish(
        &self,
        request: tonic::Request<ProblemId>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        PublishEndpoint::publish(self, request).await
    }

    async fn unpublish(
        &self,
        request: tonic::Request<ProblemId>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        PublishEndpoint::unpublish(self, request).await
    }
    type ListStream = TonicStream<ProblemInfo>;

    async fn list(
        &self,
        request: tonic::Request<ListRequest>,
    ) -> Result<tonic::Response<Self::ListStream>, tonic::Status> {
        BaseEndpoint::list(self, request).await
    }

    type SearchByTextStream = TonicStream<ProblemInfo>;
    async fn search_by_text(
        &self,
        request: tonic::Request<TextSearchRequest>,
    ) -> Result<tonic::Response<Self::SearchByTextStream>, tonic::Status> {
        BaseEndpoint::search_by_text(self, request, &[Column::Title, Column::Content]).await
    }
    type SearchByTagStream = TonicStream<ProblemInfo>;

    async fn search_by_tag(
        &self,
        request: tonic::Request<TextSearchRequest>,
    ) -> Result<tonic::Response<Self::SearchByTagStream>, tonic::Status> {
        BaseEndpoint::search_by_text(self, request, &[Column::Tags]).await
    }

    async fn full_info_by_contest(
        &self,
        request: tonic::Request<ProblemLink>,
    ) -> Result<tonic::Response<ProblemFullInfo>, tonic::Status> {
        todo!()
    }
}
