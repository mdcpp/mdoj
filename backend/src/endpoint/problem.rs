use crate::{common::problem::*, controller::ControllerCluster};

use super::define::*;

pub struct ProblemEditor;

impl Editer for ProblemEditor{
    type Require=Require;

    type Update=Update;
}

impl Editable<ProblemEditor> for ControllerCluster{
    type Error;

    async fn create(&self,request:E::Require,user:UserInfo) -> Result<i32,Self::Error> {
        todo!()
    }

    async fn update(&self,request:E::Update,user:UserInfo) -> Result<i32,Self::Error> {
        todo!()
    }

    async fn remove(&self,request:i32,user:UserInfo) -> Result<i32,Self::Error> {
        todo!()
    }
}