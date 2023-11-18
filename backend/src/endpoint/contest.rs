use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::contest_set_server::*;
use crate::grpc::backend::*;

use entity::{contest::*, *};

#[async_trait]
impl Filter for Entity {
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() {
                return Ok(query);
            }
            if perm.can_manage_contest() {
                let user_id = auth.user_id().unwrap();
                return Ok(query.filter(Column::Hoster.eq(user_id)));
            }
        }
        Err(Error::PremissionDeny("Can't write contest"))
    }
}

#[async_trait]
impl ParentalFilter for Entity {
    fn link_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() {
                return Ok(query);
            }
            if perm.can_link() {
                let user_id = auth.user_id().unwrap();
                return Ok(query.filter(Column::Hoster.eq(user_id)));
            }
        }
        Err(Error::PremissionDeny("Can't link test"))
    }
}

impl From<i32> for ContestId {
    fn from(value: i32) -> Self {
        Self { id: value }
    }
}
impl From<ContestId> for i32 {
    fn from(value: ContestId) -> Self {
        value.id
    }
}

impl From<Model> for ContestFullInfo {
    fn from(value: Model) -> Self {
        ContestFullInfo {
            info: value.clone().into(),
            content: value.content,
            hoster: value.hoster.into(),
        }
    }
}

impl From<Model> for ContestInfo {
    fn from(value: Model) -> Self {
        // ContestInfo {
        //     id: value.id.into(),
        //     title: value.title,
        //     begin: value.begin,
        //     end: value.end,
        //     need_password: value.password.is_some(),
        // }
        todo!()
    }
}

#[async_trait]
impl ContestSet for Arc<Server> {
    async fn list(
        &self,
        req: Request<ListRequest>,
    ) -> Result<Response<ListContestResponse>, Status> {
        todo!()
    }

    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListContestResponse>, Status> {
        todo!()
    }

    async fn full_info(
        &self,
        req: Request<ContestId>,
    ) -> Result<Response<ContestFullInfo>, Status> {
        todo!()
    }

    async fn create(&self, req: Request<CreateContestRequest>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn update(&self, req: Request<UpdateContestRequest>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn remove(&self, req: Request<ContestId>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn join(&self, req: Request<ContestId>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn exit(&self, req: Request<ContestId>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn rank(&self, req: Request<ListRequest>) -> Result<Response<ListRankResponse>, Status> {
        todo!()
    }
}
