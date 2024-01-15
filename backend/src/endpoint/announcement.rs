use super::endpoints::*;
use super::tools::*;

use entity::announcement::*;

impl Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() || perm.can_manage_announcement() {
                return Ok(query);
            }
        }
        Err(Error::Unauthenticated)
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (_user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() || perm.can_manage_announcement() {
            return Ok(query);
        }
        Err(Error::Unauthenticated)
    }
}

// #[async_trait]
// impl ParentalFilter for Entity {
//     fn publish_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
//         if let Some(perm) = auth.user_perm(){
//             if perm.can_root()||perm.can_manage_announcement() {
//                 return Ok(query);
//             }
//         }
//         Err(Error::PermissionDeny("Can't publish education"))
//     }

//     fn link_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
//         if let Some(perm) = auth.user_perm(){
//             if perm.can_root()||perm.can_manage_announcement() {
//                 return Ok(query);
//             }
//         }
//         Err(Error::PermissionDeny("Can't link education"))
//     }
// }

// impl From<i32> for AnnouncementId {
//     fn from(value: i32) -> Self {
//         Self { id: value }
//     }
// }

// impl From<AnnouncementId> for i32 {
//     fn from(value: AnnouncementId) -> Self {
//         value.id
//     }
// }

// impl From<Model> for EducationFullInfo{
//     fn from(value: Model) -> Self {
//         todo!()
//     }
// }

// impl From<Model> for EducationInfo{
//     fn from(value: Model) -> Self {
//         todo!()
//     }
// }

// #[async_trait]
// impl AnnouncementSet for Arc<Server>{
//     #[instrument(skip_all, level = "debug")]
//     async fn list(
//         &self,
//         req: Request<ListAnnouncementRequest>,
//     ) -> Result<Response<ListAnnouncementResponse>, Status> {
//         let (auth, req) = self.parse_request(req).await?;

//         let mut reverse = false;
//         let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
//             list_announcement_request::Request::Create(create) => {
//                 Pager::sort_search(create.sort_by(), create.reverse)
//             }
//             list_announcement_request::Request::Pager(old) => {
//                 reverse = old.reverse;
//                 <Pager<Entity> as HasParentPager<contest::Entity, Entity>>::from_raw(
//                     old.session,
//                     self,
//                 )?
//             }
//         };

//         let list = pager
//             .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
//             .await?
//             .into_iter()
//             .map(|x| x.into())
//             .collect();

//         let next_session = pager.into_raw(self);

//         Ok(Response::new(ListAnnouncementResponse { list, next_session }))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn search_by_text(
//         &self,
//         req: Request<TextSearchRequest>,
//     ) -> Result<Response<ListProblemResponse>, Status> {
//         let (auth, req) = self.parse_request(req).await?;

//         let mut reverse = false;
//         let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
//             text_search_request::Request::Text(create) => {
//                 tracing::trace!(search = create);
//                 Pager::text_search(create)
//             }
//             text_search_request::Request::Pager(old) => {
//                 reverse = old.reverse;
//                 <Pager<_> as HasParentPager<contest::Entity, Entity>>::from_raw(old.session, self)?
//             }
//         };

//         let list = pager
//             .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
//             .await?
//             .into_iter()
//             .map(|x| x.into())
//             .collect();

//         let next_session = pager.into_raw(self);

//         Ok(Response::new(ListProblemResponse { list, next_session }))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn full_info(
//         &self,
//         req: Request<ProblemId>,
//     ) -> Result<Response<ProblemFullInfo>, Status> {
//         let db = DB.get().unwrap();
//         let (auth, req) = self.parse_request(req).await?;

//         tracing::debug!(problem_id = req.id);

//         let query = Entity::read_filter(Entity::find_by_id::<i32>(req.into()), &auth)?;
//         let model = query
//             .one(db)
//             .await
//             .map_err(Into::<Error>::into)?
//             .ok_or(Error::NotInDB("problem"))?;

//         Ok(Response::new(model.into()))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn create(
//         &self,
//         req: Request<CreateProblemRequest>,
//     ) -> Result<Response<ProblemId>, Status> {
//         let db = DB.get().unwrap();
//         let (auth, req) = self.parse_request(req).await?;
//         let (user_id, perm) = auth.ok_or_default()?;

//         let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
//         if let Some(x) = self.dup.check_i32(user_id, &uuid) {
//             return Ok(Response::new(x.into()));
//         };

//         if !(perm.can_root() || perm.can_manage_problem()) {
//             return Err(Error::PermissionDeny("Can't create problem").into());
//         }

//         let mut model: ActiveModel = Default::default();
//         model.user_id = ActiveValue::Set(user_id);

//         fill_active_model!(
//             model, req.info, title, difficulty, time, memory, tags, content, match_rule, order
//         );

//         let model = model.save(db).await.map_err(Into::<Error>::into)?;

//         self.dup.store_i32(user_id, uuid, model.id.clone().unwrap());

//         tracing::debug!(id = model.id.clone().unwrap(), "problem_created");

//         Ok(Response::new(model.id.unwrap().into()))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn update(&self, req: Request<UpdateProblemRequest>) -> Result<Response<()>, Status> {
//         let db = DB.get().unwrap();
//         let (auth, req) = self.parse_request(req).await?;

//         let (user_id, _perm) = auth.ok_or_default()?;

//         let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
//         if self.dup.check_i32(user_id, &uuid).is_some() {
//             return Ok(Response::new(()));
//         };

//         tracing::trace!(id = req.id.id);

//         let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
//             .one(db)
//             .await
//             .map_err(Into::<Error>::into)?
//             .ok_or(Error::NotInDB("problem"))?
//             .into_active_model();

//         fill_exist_active_model!(
//             model,
//             req.info,
//             title,
//             difficulty,
//             time,
//             memory,
//             tags,
//             content,
//             match_rule,
//             ac_rate,
//             submit_count,
//             order
//         );

//         let model = model.update(db).await.map_err(Into::<Error>::into)?;

//         self.dup.store_i32(user_id, uuid, model.id);

//         Ok(Response::new(()))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn remove(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
//         let db = DB.get().unwrap();
//         let (auth, req) = self.parse_request(req).await?;

//         Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
//             .exec(db)
//             .await
//             .map_err(Into::<Error>::into)?;

//         tracing::debug!(id = req.id);

//         Ok(Response::new(()))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn add_to_contest(&self, req: Request<AddProblemToContestRequest>) -> Result<Response<()>, Status> {
//         let db = DB.get().unwrap();
//         let (auth, req) = self.parse_request(req).await?;

//         let (_, perm) = auth.ok_or_default()?;

//         if !(perm.can_root() || perm.can_link()) {
//             return Err(Error::PermissionDeny("Can't link problem").into());
//         }

//         let mut problem = Entity::link_filter(Entity::find_by_id(req.problem_id), &auth)?
//             .columns([Column::Id, Column::ContestId])
//             .one(db)
//             .await
//             .map_err(Into::<Error>::into)?
//             .ok_or(Error::NotInDB("problem"))?
//             .into_active_model();

//         problem.contest_id = ActiveValue::Set(Some(req.contest_id.id));

//         problem.save(db).await.map_err(Into::<Error>::into)?;

//         Ok(Response::new(()))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn remove_from_contest(&self, req: Request<AddProblemToContestRequest>) -> Result<Response<()>, Status> {
//         let db = DB.get().unwrap();
//         let (auth, req) = self.parse_request(req).await?;

//         let (_, perm) = auth.ok_or_default()?;

//         if !(perm.can_root() || perm.can_link()) {
//             return Err(Error::PermissionDeny("Can't link problem").into());
//         }

//         let mut problem = Entity::link_filter(Entity::find_by_id(req.problem_id), &auth)?
//             .columns([Column::Id, Column::ContestId])
//             .one(db)
//             .await
//             .map_err(Into::<Error>::into)?
//             .ok_or(Error::NotInDB("problem"))?
//             .into_active_model();

//         problem.contest_id = ActiveValue::Set(None);

//         problem.save(db).await.map_err(Into::<Error>::into)?;

//         Ok(Response::new(()))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn publish(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
//         let db = DB.get().unwrap();
//         let (auth, req) = self.parse_request(req).await?;

//         auth.ok_or_default()?;

//         tracing::debug!(id = req.id);

//         let mut problem =
//             Entity::publish_filter(Entity::find_by_id(Into::<i32>::into(req)), &auth)?
//                 .columns([Column::Id, Column::ContestId])
//                 .one(db)
//                 .await
//                 .map_err(Into::<Error>::into)?
//                 .ok_or(Error::NotInDB("problem"))?
//                 .into_active_model();

//         problem.public = ActiveValue::Set(true);

//         problem.save(db).await.map_err(Into::<Error>::into)?;

//         Ok(Response::new(()))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn unpublish(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
//         let db = DB.get().unwrap();
//         let (auth, req) = self.parse_request(req).await?;

//         auth.ok_or_default()?;

//         tracing::debug!(id = req.id);

//         let mut problem =
//             Entity::publish_filter(Entity::find_by_id(Into::<i32>::into(req)), &auth)?
//                 .columns([Column::Id, Column::ContestId])
//                 .one(db)
//                 .await
//                 .map_err(Into::<Error>::into)?
//                 .ok_or(Error::NotInDB("problem"))?
//                 .into_active_model();

//         problem.public = ActiveValue::Set(false);

//         problem.save(db).await.map_err(Into::<Error>::into)?;

//         Ok(Response::new(()))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn full_info_by_contest(
//         &self,
//         req: Request<AddProblemToContestRequest>,
//     ) -> Result<Response<ProblemFullInfo>, Status> {
//         let db = DB.get().unwrap();
//         let (auth, req) = self.parse_request(req).await?;

//         let parent = auth
//             .get_user(db)
//             .await?
//             .find_related(contest::Entity)
//             .columns([contest::Column::Id])
//             .one(db)
//             .await
//             .map_err(Into::<Error>::into)?
//             .ok_or(Error::NotInDB("contest"))?;

//         let model = parent
//             .find_related(Entity)
//             .filter(Column::Id.eq(Into::<i32>::into(req.problem_id)))
//             .one(db)
//             .await
//             .map_err(Into::<Error>::into)?
//             .ok_or(Error::NotInDB("problem"))?;

//         Ok(Response::new(model.into()))
//     }
//     #[instrument(skip_all, level = "debug")]
//     async fn list_by_contest(
//         &self,
//         req: Request<ListByRequest>,
//     ) -> Result<Response<ListProblemResponse>, Status> {
//         let (auth, req) = self.parse_request(req).await?;

//         let mut reverse = false;
//         let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
//             list_by_request::Request::ParentId(ppk) => {
//                 tracing::debug!(id = ppk);
//                 Pager::parent_sorted_search(ppk, ProblemSortBy::Order, false)
//             }
//             list_by_request::Request::Pager(old) => {
//                 reverse = old.reverse;
//                 <Pager<Entity> as HasParentPager<contest::Entity, Entity>>::from_raw(
//                     old.session,
//                     self,
//                 )?
//             }
//         };

//         let list = pager
//             .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
//             .await?
//             .into_iter()
//             .map(|x| x.into())
//             .collect();

//         let next_session = pager.into_raw(self);

//         Ok(Response::new(ListProblemResponse { list, next_session }))
//     }
// }