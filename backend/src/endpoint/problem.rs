use tonic::async_trait;

use crate::{common::problem::*, controller::ControllerCluster};

use super::define::*;

pub struct ProblemEditor;

impl Editer for ProblemEditor {
    type Require = Require;

    type Update = Update;
}

#[async_trait]
impl Editable<ProblemEditor> for ControllerCluster {
    type Error = super::Error;

    async fn create(
        &self,
        request: <ProblemEditor as Editer>::Require,
        user: UserInfo,
    ) -> Result<i32, Self::Error> {
        todo!()
    }

    async fn update(
        &self,
        request: <ProblemEditor as Editer>::Update,
        user: UserInfo,
    ) -> Result<i32, Self::Error> {
        todo!()
    }

    async fn remove(&self, request: i32, user: UserInfo) -> Result<i32, Self::Error> {
        todo!()
    }
}
