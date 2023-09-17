use std::pin::Pin;

use crate::{endpoint::*, grpc::proto::prelude::*, init::db::DB, Server};

use super::util::intel::*;
use tonic::{Request, Response};

use entity::problem::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Select, ActiveValue};
pub struct ProblemIntel;

impl IntelTrait for ProblemIntel {
    type Entity = Entity;

    type PartialModel = PartialProblem;

    type InfoArray = Problems;

    type FullInfo = ProblemFullInfo;

    type Info = ProblemInfo;

    // const INFO_INTERESTS:
    //     &'static [<<Self as IntelTrait>::Entity as sea_orm::EntityTrait>::Column] =
    //     &[Column::Title, Column::Id, Column::Submits, Column::AcRate];
}

impl Intel<ProblemIntel> for Server {
    fn ro_filter(
        self_: Select<<ProblemIntel as IntelTrait>::Entity>,
        auth: super::Auth,
    ) -> Result<Select<<ProblemIntel as IntelTrait>::Entity>, tonic::Status> {
        Ok(match auth {
            Auth::Guest => self_.filter(Column::Public.eq(true)),
            Auth::User((user_id, perm)) => match perm.can_root() || perm.can_manage_problem() {
                true => self_,
                false => self_.filter(Column::Public.eq(true).or(Column::UserId.eq(user_id))),
            },
        })
    }
}

impl Endpoint<ProblemIntel> for Server {}

impl Transform<<Entity as EntityTrait>::Column> for SortBy {
    fn into(self) -> <<ProblemIntel as IntelTrait>::Entity as EntityTrait>::Column {
        match self {
            SortBy::SubmitCount => Column::Submits,
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
                submits: x.submits,
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
            submits: self.submits,
            ac_rate: self.ac_rate,
        }
    }
}

impl Transform<ProblemFullInfo> for Model {
    fn into(self) -> ProblemFullInfo {
        todo!()
    }
}

impl Transform<i32> for ProblemId {
    fn into(self) -> i32 {
        todo!()
    }
}

#[async_trait]
impl problem_set_server::ProblemSet for Server {
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        Endpoint::list(self, request).await
    }

    async fn search_by_text(
        &self,
        request: Request<TextSearchRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        Endpoint::search_by_text(self, request, &[Column::Title, Column::Content]).await
    }

    async fn search_by_tag(
        &self,
        request: Request<TextSearchRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        Endpoint::search_by_text(self, request, &[Column::Tags]).await
    }

    async fn full_info(
        &self,
        request: Request<ProblemId>,
    ) -> Result<Response<ProblemFullInfo>, tonic::Status> {
        Endpoint::full_info(self, request).await
    }

    async fn create(
        &self,
        request: tonic::Request<ProblemFullInfo>,
    ) -> Result<Response<()>, tonic::Status> {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        match auth {
            Auth::Guest => Err(tonic::Status::permission_denied("Guest cannot create")),
            Auth::User((user_id, perm)) => {
                if perm.can_root() || perm.can_manage_problem() {
                    ActiveModel {
                        user_id: ActiveValue::NotSet,
                        contest_id: ActiveValue::NotSet,
                        success: ActiveValue::Set(0),
                        submits: ActiveValue::Set(0),
                        ac_rate: ActiveValue::Set(1.0),
                        memory: todo!(),
                        time: todo!(),
                        difficulty: todo!(),
                        public: todo!(),
                        tags: todo!(),
                        title: todo!(),
                        content: todo!(),
                        ..Default::default()
                    };
                    todo!()
                } else {
                    Err(tonic::Status::permission_denied("User cannot create"))
                }
            }
        }
    }

    async fn update(
        &self,
        request: tonic::Request<ProblemFullInfo>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
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
