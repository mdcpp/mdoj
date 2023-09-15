// use crate::grpc::proto::*;

use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use tonic::{async_trait, Request, Response};

use crate::common::error::*;

use crate::{
    endpoint::util::permission::Auth, grpc::proto::prelude::problem_set_server::ProblemSet,
    grpc::proto::prelude::*, init::db::DB, Server,
};

// pub enum ProblemSort {
//     ACRate,
//     UploadDate,
//     SubmitCount,
//     Score,
//     Difficulty,
// }

impl Into<entity::problem::Column> for SortBy {
    fn into(self) -> entity::problem::Column {
        match self {
            Self::AcRate => entity::problem::Column::AcRate,
            Self::UploadDate => entity::problem::Column::Id,
            Self::SubmitCount => entity::problem::Column::Submits,
            Self::Score => entity::problem::Column::Id,
            Self::Difficulty => entity::problem::Column::Difficulty,
        }
    }
}

impl Into<ProblemInfo> for entity::problem::Model {
    fn into(self) -> ProblemInfo {
        ProblemInfo {
            id: Some(ProblemId { id: self.id }),
            title: self.title,
            ac_rate: (self.success as f32) / (self.submits as f32),
            submit_count: self.submits,
        }
    }
}

#[async_trait]
impl ProblemSet for Server {
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        let db = DB.get().unwrap();
        let (auth, request) = self.parse_request(request).await?;

        let mut filtered = match auth {
            Auth::Guest => {
                entity::problem::Entity::find().filter(entity::problem::Column::Visible.eq(true))
            }
            Auth::User((user_id, perm)) => {
                if perm.can_root() {
                    entity::problem::Entity::find()
                } else {
                    entity::problem::Entity::find().filter(
                        entity::problem::Column::Visible
                            .eq(true)
                            .or(entity::problem::Column::UserId.eq(user_id)),
                    )
                }
            }
        };

        let offset = request.page.as_ref().map(|x| x.offset).unwrap_or(0);
        let limit = request.page.map(|x| x.amount).unwrap_or(10);
        let reversed = request.sort.as_ref().map(|x| x.reverse).unwrap_or(false);

        let sort_col = match request.sort {
            Some(x) => SortBy::from_i32(x.sort_by)
                .ok_or(tonic::Status::invalid_argument(
                    "SortBy is not a vaild emun",
                ))?
                .into(),
            None => entity::problem::Column::Id,
        };

        if reversed {
            filtered = filtered.order_by_desc(sort_col);
        } else {
            filtered = filtered.order_by_asc(sort_col);
        }

        let filtered = filtered.offset(offset as u64).limit(limit as u64);

        let list = result_into(filtered.all(db).await)?
            .into_iter()
            .map(|x| x.into())
            .collect();
        Ok(Response::new(Problems { list }))
    }

    async fn search_by_text(
        &self,
        request: Request<SearchByTextRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        todo!()
    }

    async fn search_by_tag(
        &self,
        request: Request<SearchByTagRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        todo!()
    }

    async fn full_info(
        &self,
        request: Request<ProblemId>,
    ) -> Result<Response<ProblemFullInfo>, tonic::Status> {
        todo!()
    }

    async fn create(
        &self,
        request: Request<ProblemFullInfo>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn update(
        &self,
        request: Request<ProblemFullInfo>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn remove(&self, request: Request<ProblemId>) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn link(&self, request: Request<ProblemLink>) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn unlink(&self, request: Request<ProblemLink>) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn add_test(&self, request: Request<Testcase>) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn remove_test(
        &self,
        request: Request<TestcaseId>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }
}
