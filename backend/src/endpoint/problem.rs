// use super::endpoints::*;
// use super::tools::*;

// use crate::fill_active_model;
// use crate::fill_exist_active_model;

// use crate::{endpoint::*, grpc::backend::*, impl_id, Server};

// use entity::{problem::*, *};
// use tonic::*;

// type TonicStream<T> = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<T, Status>> + Send>>;

// impl_endpoint!(Problem);

// #[async_trait]
// impl Intel<ProblemIntel> for Server {
//     fn ro_filter<S>(query: S, auth: Auth) -> Result<S, Error>
//     where
//         S: QueryFilter,
//     {
//         Ok(match auth {
//             Auth::Guest => query.filter(Column::Public.eq(true)),
//             Auth::User((user_id, perm)) => match perm.can_manage_problem() {
//                 true => query,
//                 false => query.filter(Column::Public.eq(true).or(Column::UserId.eq(user_id))),
//             },
//         })
//     }

//     fn rw_filter<S>(query: S, auth: Auth) -> Result<S, Error>
//     where
//         S: QueryFilter,
//     {
//         let (user_id, perm) = auth.ok_or_default()?;
//         if perm.can_manage_problem() {
//             Ok(query.filter(Column::UserId.eq(user_id)))
//         } else {
//             Err(Error::PremissionDeny(
//                 "Only User with `can_manage_problem` can modify problem",
//             ))
//         }
//     }

//     fn can_create(auth: Auth) -> Result<i32, Error> {
//         let (user_id, perm) = auth.ok_or_default()?;
//         match perm.can_manage_problem() {
//             true => Ok(user_id),
//             false => Err(Error::PremissionDeny(
//                 "Only User with `can_manage_problem` can create problem",
//             )),
//         }
//     }

//     async fn update_model(model: Model, info: update_problem_request::Info) -> Result<(), Error> {
//         let db = DB.get().unwrap();

//         let mut target = model.into_active_model();
//         fill_exist_active_model!(
//             target, info, title, content, memory, time, difficulty, tags, match_rule
//         );

//         target.save(db).await?;
//         Ok(())
//     }

//     async fn create_model(info: create_problem_request::Info, user_id: i32) -> Result<i32, Error> {
//         let db = DB.get().unwrap();

//         let mut model: ActiveModel = Default::default();
//         fill_active_model!(model, info, memory, time, difficulty, tags, title, content, match_rule);
//         model.user_id = ActiveValue::Set(user_id);

//         let model = model.insert(db).await?;
//         Ok(model.id)
//     }
// }

// impl Transform<<Entity as EntityTrait>::Column> for SortBy {
//     fn into(self) -> <<ProblemIntel as IntelTrait>::Entity as EntityTrait>::Column {
//         match self {
//             SortBy::SubmitCount => Column::SubmitCount,
//             SortBy::AcRate => Column::AcRate,
//             SortBy::Difficulty => Column::Difficulty,
//             _ => Column::Id,
//         }
//     }
// }

// impl Transform<Problems> for Vec<ProblemInfo> {
//     fn into(self) -> Problems {
//         let list = self
//             .into_iter()
//             .map(|x| ProblemInfo {
//                 id: x.id,
//                 title: x.title,
//                 submit_count: x.submit_count,
//                 ac_rate: x.ac_rate,
//             })
//             .collect();
//         Problems { list }
//     }
// }

// impl Transform<<ProblemIntel as IntelTrait>::Info> for PartialProblem {
//     fn into(self) -> <ProblemIntel as IntelTrait>::Info {
//         ProblemInfo {
//             id: Some(Transform::into(self.id)),
//             title: self.title,
//             submit_count: self.submit_count,
//             ac_rate: self.ac_rate,
//         }
//     }
// }

// // TODO, use specialized impl under ``Intel`` for such (operation?)
// #[async_trait]
// impl AsyncTransform<Result<ProblemFullInfo, Error>> for Model {
//     async fn into(self) -> Result<ProblemFullInfo, Error> {
//         let db = DB.get().unwrap();

//         let (edu, test) = tokio::join!(
//             self.find_related(education::Entity)
//                 .select_only()
//                 .columns([education::Column::Id])
//                 .one(db),
//             self.find_related(test::Entity)
//                 .select_only()
//                 .columns([test::Column::Id])
//                 .all(db)
//         );

//         let education_id = edu?.map(|x| EducationId { id: x.id });
//         let test_id: Vec<TestcaseId> = test?.into_iter().map(|x| TestcaseId { id: x.id }).collect();

//         Ok(ProblemFullInfo {
//             content: self.content,
//             memory: self.memory,
//             time: self.time,
//             difficulty: self.difficulty,
//             tags: self.tags,
//             public: self.public,
//             info: Some(ProblemInfo {
//                 id: Some(ProblemId { id: self.id }),
//                 title: self.title,
//                 submit_count: self.submit_count,
//                 ac_rate: self.ac_rate,
//             }),
//             education_id,
//             testcases: Some(Testcases { list: test_id }),
//         })
//     }
// }

// // pub struct ProblemLink;

// // impl LinkTrait for ProblemLink {
// //     type Linker = ProblemLink;
// //     type Intel = ProblemIntel;

// //     type ParentIntel;
// // }

// impl PublishTrait<ProblemIntel> for ProblemIntel {
//     type Publisher = ProblemId;
// }

// #[async_trait]
// impl Publishable<ProblemIntel> for Server {
//     async fn publish(&self, entity: Model) -> Result<(), Error> {
//         let db = DB.get().unwrap();

//         let mut model = entity.into_active_model();
//         model.public = ActiveValue::Set(true);

//         model.save(db).await?;

//         Ok(())
//     }

//     async fn unpublish(&self, entity: Model) -> Result<(), Error> {
//         let db = DB.get().unwrap();

//         let mut model = entity.into_active_model();
//         model.public = ActiveValue::Set(false);

//         model.save(db).await?;

//         Ok(())
//     }
// }

// // impl Endpoints
// impl BaseEndpoint<ProblemIntel> for Server {}
// impl PublishEndpoint<ProblemIntel> for Server {}

// // Adapters
// #[async_trait]
// impl problem_set_server::ProblemSet for Server {
//     async fn full_info(
//         &self,
//         request: Request<ProblemId>,
//     ) -> Result<Response<ProblemFullInfo>, Status> {
//         BaseEndpoint::<ProblemIntel>::full_info(self, request)
//             .await
//             .map_err(|x| x.into())
//     }

//     async fn create(
//         &self,
//         request: tonic::Request<CreateProblemRequest>,
//     ) -> Result<Response<ProblemId>, Status> {
//         BaseEndpoint::<ProblemIntel>::create(self, request)
//             .await
//             .map_err(|x| x.into())
//     }

//     async fn update(
//         &self,
//         request: tonic::Request<UpdateProblemRequest>,
//     ) -> Result<Response<()>, Status> {
//         BaseEndpoint::<ProblemIntel>::update(self, request)
//             .await
//             .map_err(|x| x.into())
//     }

//     async fn remove(&self, request: tonic::Request<ProblemId>) -> Result<Response<()>, Status> {
//         BaseEndpoint::<ProblemIntel>::remove(self, request)
//             .await
//             .map_err(|x| x.into())
//     }

//     async fn link(&self, request: tonic::Request<ProblemLink>) -> Result<Response<()>, Status> {
//         // let db = DB.get().unwrap();

//         // let (auth, payload) = self.parse_request(request).await?;

//         // let contest_id = parse_option!(payload, contest_id).id;
//         // let problem_id = parse_option!(payload, problem_id).id;

//         // let (_, perm) = auth.ok_or_default()?;

//         // if perm.can_link() {
//         //     let mut problem = handle_dberr(
//         //         Entity::find_by_id(problem_id)
//         //             .select_only()
//         //             .columns([Column::ContestId])
//         //             .one(db)
//         //             .await,
//         //     )?
//         //     .ok_or(Status::not_found("problem not found"))?
//         //     .into_active_model();
//         //     problem.contest_id = ActiveValue::Set(contest_id);
//         //     handle_dberr(problem.update(db).await).map(|_| Response::new(()))
//         // } else {
//         //     Err(Status::permission_denied("User cannot link"))
//         // }
//         todo!()
//     }

//     async fn unlink(&self, request: tonic::Request<ProblemLink>) -> Result<Response<()>, Status> {
//         // let (auth, payload) = self.parse_request(request).await?;

//         // let (_, perm) = auth.ok_or_default()?;

//         // if perm.can_root() || perm.can_link() {
//         //     let db = DB.get().unwrap();
//         //     let contest_id = payload
//         //         .contest_id
//         //         .ok_or(Status::not_found("contest id not found"))?
//         //         .id;
//         //     let problem_id = payload
//         //         .problem_id
//         //         .ok_or(Status::not_found("problem id not found"))?
//         //         .id;
//         //     let mut problem = handle_dberr(
//         //         Entity::find_by_id(problem_id)
//         //             .select_only()
//         //             .columns([Column::ContestId])
//         //             .one(db)
//         //             .await,
//         //     )?
//         //     .ok_or(Status::not_found("problem not found"))?
//         //     .into_active_model();
//         //     problem.contest_id = ActiveValue::Set(contest_id);
//         //     handle_dberr(problem.update(db).await).map(|_| Response::new(()))
//         // } else {
//         //     Err(Status::permission_denied(""))
//         // }
//         todo!()
//     }

//     async fn publish(
//         &self,
//         request: tonic::Request<ProblemId>,
//     ) -> Result<tonic::Response<()>, Status> {
//         PublishEndpoint::<ProblemIntel>::publish(self, request)
//             .await
//             .map_err(|x| x.into())
//     }

//     async fn unpublish(
//         &self,
//         request: tonic::Request<ProblemId>,
//     ) -> Result<tonic::Response<()>, Status> {
//         PublishEndpoint::<ProblemIntel>::unpublish(self, request)
//             .await
//             .map_err(|x| x.into())
//     }
//     type ListStream = TonicStream<ProblemInfo>;

//     async fn list(
//         &self,
//         request: tonic::Request<ListRequest>,
//     ) -> Result<tonic::Response<Self::ListStream>, Status> {
//         BaseEndpoint::<ProblemIntel>::list(self, request)
//             .await
//             .map_err(|x| x.into())
//     }

//     type SearchByTextStream = TonicStream<ProblemInfo>;
//     async fn search_by_text(
//         &self,
//         request: tonic::Request<TextSearchRequest>,
//     ) -> Result<tonic::Response<Self::SearchByTextStream>, Status> {
//         BaseEndpoint::<ProblemIntel>::search_by_text(
//             self,
//             request,
//             &[Column::Title, Column::Content],
//         )
//         .await
//         .map_err(|x| x.into())
//     }
//     type SearchByTagStream = TonicStream<ProblemInfo>;

//     async fn search_by_tag(
//         &self,
//         request: tonic::Request<TextSearchRequest>,
//     ) -> Result<tonic::Response<Self::SearchByTagStream>, Status> {
//         BaseEndpoint::<ProblemIntel>::search_by_text(self, request, &[Column::Tags])
//             .await
//             .map_err(|x| x.into())
//     }

//     async fn full_info_by_contest(
//         &self,
//         request: tonic::Request<ProblemLink>,
//     ) -> Result<tonic::Response<ProblemFullInfo>, Status> {
//         todo!()
//     }

//     #[doc = " Server streaming response type for the ListByContest method."]
//     type ListByContestStream = TonicStream<ProblemInfo>;

//     async fn list_by_contest(
//         &self,
//         request: tonic::Request<ContestId>,
//     ) -> Result<tonic::Response<Self::ListByContestStream>, Status> {
//         todo!()
//     }
// }
