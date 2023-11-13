use super::endpoints::*;
use super::tools::*;

use crate::{endpoint::*, grpc::prelude::*, impl_id, Server};

use entity::{problem::*, *};
use tonic::*;

type TonicStream<T> = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<T, Status>> + Send>>;

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

    type PartialModel = PartialTestcase;

    type InfoArray = Problems;

    type FullInfo = ProblemFullInfo;

    type Info = ProblemInfo;

    type PrimaryKey = i32;

    type Id = ProblemId;

    type UpdateInfo = update_problem_request::Info;

    type CreateInfo = create_problem_request::Info;

    const NAME: &'static str = "problem";
}

#[async_trait]
impl Intel<ProblemIntel> for Server {
    fn ro_filter<S>(query: S, auth: Auth) -> Result<S, Error>
    where
        S: QueryFilter,
    {
        Ok(match auth {
            Auth::Guest => query.filter(Column::Public.eq(true)),
            Auth::User((user_id, perm)) => match perm.can_manage_problem() {
                true => query,
                false => query.filter(Column::Public.eq(true).or(Column::UserId.eq(user_id))),
            },
        })
    }

    fn rw_filter<S>(query: S, auth: Auth) -> Result<S, Error>
    where
        S: QueryFilter,
    {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_manage_problem() {
            Ok(query.filter(Column::UserId.eq(user_id)))
        } else {
            Err(Error::PremissionDeny(
                "Only User with `can_manage_problem` can modify problem",
            ))
        }
    }

    fn can_create(auth: Auth) -> Result<i32, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        match perm.can_manage_problem() {
            true => Ok(user_id),
            false => Err(Error::PremissionDeny(
                "Only User with `can_manage_problem` can create problem",
            )),
        }
    }

    async fn update_model(model: Model, info: update_problem_request::Info) -> Result<i32, Error> {
        let db = DB.get().unwrap();
        let user_id = model.user_id;

        let mut target = model.into_active_model();
        insert_if_exists!(target, info, title, content, memory, time, difficulty, tags,match_rule);

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

            let list = vr_to_rv(futures::future::join_all(futs).await)?;

            let vaild = list
                .into_iter()
                .map(|x| x == 0)
                .reduce(|x, y| x || y)
                .unwrap_or(true);

            if vaild {
                todo!("Update testcases's problem_id")
            } else {
                return Err(Error::PremissionDeny("Only admin can add unowned testcase"));
            };
        };

        todo!("commit changes(active model)")
    }
 
    async fn create_model(model: create_problem_request::Info, user_id: i32) -> Result<i32, Error> {
        let db = DB.get().unwrap();

        let model = ActiveModel {
            user_id: ActiveValue::Set(user_id),
            ac_rate: ActiveValue::Set(0.0),
            memory: ActiveValue::Set(model.memory),
            time: ActiveValue::Set(model.time),
            difficulty: ActiveValue::Set(model.difficulty),
            tags: ActiveValue::Set(model.tags),
            title: ActiveValue::Set(model.title),
            content: ActiveValue::Set(model.content),
            match_rule: ActiveValue::Set(model.match_rule),
            ..Default::default()
        }
        .insert(db)
        .await?;
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

impl Transform<<ProblemIntel as IntelTrait>::Info> for PartialTestcase {
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
impl AsyncTransform<Result<ProblemFullInfo, Error>> for Model {
    async fn into(self) -> Result<ProblemFullInfo, Error> {
        let db = DB.get().unwrap();

        let edu = self
            .find_related(education::Entity)
            .select_only()
            .columns([education::Column::Id])
            .one(db)
            .await?;
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

impl TryTransform<create_problem_request::Info, Error> for CreateProblemRequest {
    fn try_into(self) -> Result<create_problem_request::Info, Error> {
        let info = self.info.ok_or(Error::NotInPayload("info"))?;
        Ok(info)
    }
}

impl TryTransform<(update_problem_request::Info, i32), Error> for UpdateProblemRequest {
    fn try_into(self) -> Result<(update_problem_request::Info, i32), Error> {
        let info = self.info.ok_or(Error::NotInPayload("info"))?;
        let id = self.id.map(|x| x.id).ok_or(Error::NotInPayload("id"))?;
        Ok((info, id))
    }
}

// pub struct ProblemLink;

// impl LinkTrait for ProblemLink {
//     type Linker = ProblemLink;
//     type Intel = ProblemIntel;

//     type ParentIntel;
// }

impl PublishTrait<ProblemIntel> for ProblemIntel {
    type Publisher = ProblemId;
}

#[async_trait]
impl Publishable<ProblemIntel> for Server {
    async fn publish(&self, entity: Model) -> Result<(), Error> {
        let db = DB.get().unwrap();

        let mut model = entity.into_active_model();
        model.public = ActiveValue::Set(true);

        model.save(db).await?;

        Ok(())
    }

    async fn unpublish(&self, entity: Model) -> Result<(), Error> {
        let db = DB.get().unwrap();

        let mut model = entity.into_active_model();
        model.public = ActiveValue::Set(false);

        model.save(db).await?;

        Ok(())
    }
}

// impl Endpoints
impl BaseEndpoint<ProblemIntel> for Server {}
impl PublishEndpoint<ProblemIntel> for Server {}

// Adapters
#[async_trait]
impl problem_set_server::ProblemSet for Server {
    async fn full_info(
        &self,
        request: Request<ProblemId>,
    ) -> Result<Response<ProblemFullInfo>, Status> {
        BaseEndpoint::<ProblemIntel>::full_info(self, request)
            .await
            .map_err(|x| x.into())
    }

    async fn create(
        &self,
        request: tonic::Request<CreateProblemRequest>,
    ) -> Result<Response<ProblemId>, Status> {
        BaseEndpoint::<ProblemIntel>::create(self, request)
            .await
            .map_err(|x| x.into())
    }

    async fn update(
        &self,
        request: tonic::Request<UpdateProblemRequest>,
    ) -> Result<Response<()>, Status> {
        BaseEndpoint::<ProblemIntel>::update(self, request)
            .await
            .map_err(|x| x.into())
    }

    async fn remove(&self, request: tonic::Request<ProblemId>) -> Result<Response<()>, Status> {
        BaseEndpoint::<ProblemIntel>::remove(self, request)
            .await
            .map_err(|x| x.into())
    }

    async fn link(&self, request: tonic::Request<ProblemLink>) -> Result<Response<()>, Status> {
        // let db = DB.get().unwrap();

        // let (auth, payload) = self.parse_request(request).await?;

        // let contest_id = parse_option!(payload, contest_id).id;
        // let problem_id = parse_option!(payload, problem_id).id;

        // let (_, perm) = auth.ok_or_default()?;

        // if perm.can_link() {
        //     let mut problem = handle_dberr(
        //         Entity::find_by_id(problem_id)
        //             .select_only()
        //             .columns([Column::ContestId])
        //             .one(db)
        //             .await,
        //     )?
        //     .ok_or(Status::not_found("problem not found"))?
        //     .into_active_model();
        //     problem.contest_id = ActiveValue::Set(contest_id);
        //     handle_dberr(problem.update(db).await).map(|_| Response::new(()))
        // } else {
        //     Err(Status::permission_denied("User cannot link"))
        // }
        todo!()
    }

    async fn unlink(&self, request: tonic::Request<ProblemLink>) -> Result<Response<()>, Status> {
        // let (auth, payload) = self.parse_request(request).await?;

        // let (_, perm) = auth.ok_or_default()?;

        // if perm.can_root() || perm.can_link() {
        //     let db = DB.get().unwrap();
        //     let contest_id = payload
        //         .contest_id
        //         .ok_or(Status::not_found("contest id not found"))?
        //         .id;
        //     let problem_id = payload
        //         .problem_id
        //         .ok_or(Status::not_found("problem id not found"))?
        //         .id;
        //     let mut problem = handle_dberr(
        //         Entity::find_by_id(problem_id)
        //             .select_only()
        //             .columns([Column::ContestId])
        //             .one(db)
        //             .await,
        //     )?
        //     .ok_or(Status::not_found("problem not found"))?
        //     .into_active_model();
        //     problem.contest_id = ActiveValue::Set(contest_id);
        //     handle_dberr(problem.update(db).await).map(|_| Response::new(()))
        // } else {
        //     Err(Status::permission_denied(""))
        // }
        todo!()
    }

    async fn publish(
        &self,
        request: tonic::Request<ProblemId>,
    ) -> Result<tonic::Response<()>, Status> {
        PublishEndpoint::<ProblemIntel>::publish(self, request)
            .await
            .map_err(|x| x.into())
    }

    async fn unpublish(
        &self,
        request: tonic::Request<ProblemId>,
    ) -> Result<tonic::Response<()>, Status> {
        PublishEndpoint::<ProblemIntel>::unpublish(self, request)
            .await
            .map_err(|x| x.into())
    }
    type ListStream = TonicStream<ProblemInfo>;

    async fn list(
        &self,
        request: tonic::Request<ListRequest>,
    ) -> Result<tonic::Response<Self::ListStream>, Status> {
        BaseEndpoint::<ProblemIntel>::list(self, request)
            .await
            .map_err(|x| x.into())
    }

    type SearchByTextStream = TonicStream<ProblemInfo>;
    async fn search_by_text(
        &self,
        request: tonic::Request<TextSearchRequest>,
    ) -> Result<tonic::Response<Self::SearchByTextStream>, Status> {
        BaseEndpoint::<ProblemIntel>::search_by_text(
            self,
            request,
            &[Column::Title, Column::Content],
        )
        .await
        .map_err(|x| x.into())
    }
    type SearchByTagStream = TonicStream<ProblemInfo>;

    async fn search_by_tag(
        &self,
        request: tonic::Request<TextSearchRequest>,
    ) -> Result<tonic::Response<Self::SearchByTagStream>, Status> {
        BaseEndpoint::<ProblemIntel>::search_by_text(self, request, &[Column::Tags])
            .await
            .map_err(|x| x.into())
    }

    async fn full_info_by_contest(
        &self,
        request: tonic::Request<ProblemLink>,
    ) -> Result<tonic::Response<ProblemFullInfo>, Status> {
        todo!()
    }

    #[doc = " Server streaming response type for the ListByContest method."]
    type ListByContestStream = TonicStream<ProblemInfo>;

    async fn list_by_contest(
        &self,
        request: tonic::Request<ContestId>,
    ) -> Result<tonic::Response<Self::ListByContestStream>, Status> {
        todo!()
    }
}
