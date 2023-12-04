use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::playground_set_server::*;
use crate::grpc::backend::*;

#[async_trait]
impl PlaygroundSet for Arc<Server> {
    #[doc = " Server streaming response type for the Run method."]
    type RunStream = TonicStream<PlaygroundResult>;

    async fn run(
        &self,
        req: Request<PlaygroundRequest>,
    ) -> Result<Response<Self::RunStream>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }
}
