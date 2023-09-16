use crate::{endpoint::*, grpc::proto::prelude::*, Server};

use super::util::intel::*;
use tonic::{Request, Response};

use entity::problem::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Select};
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
            Auth::User((user_id, perm)) => match !perm.can_root() {
                true => self_.filter(Column::Public.eq(true).or(Column::UserId.eq(user_id))),
                false => self_,
            },
        })
    }
}

impl Endpoint<ProblemIntel> for Server {}

impl Transform<<Entity as EntityTrait>::Column> for SortBy {
    fn into(self) -> <<ProblemIntel as IntelTrait>::Entity as EntityTrait>::Column {
        todo!()
    }
}

impl Transform<Problems> for Vec<ProblemInfo> {
    fn into(self) -> Problems {
        todo!()
    }
}

impl Transform<<ProblemIntel as IntelTrait>::Info> for PartialProblem {
    fn into(self) -> <ProblemIntel as IntelTrait>::Info {
        todo!()
    }
}

// #[async_trait]
// impl problem_set_server::ProblemSet for Server {
//     async fn list(
//         &self,
//         request: Request<ListRequest>,
//     ) -> Result<Response<Problems>, tonic::Status> {
//         Endpoint::list(self, request).await
//     }

//     async fn search_by_text(
//         &self,
//         request: Request<SearchByTextRequest>,
//     ) -> Result<Response<Problems>, tonic::Status> {
//         Endpoint::search_by_text(self, request, &[Column::Title,Column::Content]).await
//     }

//     async fn search_by_tag(
//         &self,
//         request: Request<SearchByTagRequest>,
//     ) -> Result<Response<Problems>, tonic::Status> {
//         Endpoint::search_by_text(self, request, &[Column::Tags]).await
//     }

//     // async fn full_info(
//     //     &self,
//     //     request: Request<ProblemId>,
//     // ) -> Result<Response<ProblemFullInfo>, tonic::Status> {
//     //     todo!()
//     // }
// }
