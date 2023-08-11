use std::{sync::atomic::{AtomicUsize,Ordering}, net::SocketAddr};

use tonic::transport::Uri;

use crate::grpc::proto::prelude::judger_client::JudgerClient;

struct JudgeRouter{
    sequence:AtomicUsize,
    servers:Vec<Uri>
}

impl JudgeRouter {
    async fn send(&self)->Result<(),super::super::Error>{
        let sec=self.sequence.fetch_add(1, Ordering::Relaxed);

        let server=JudgerClient::connect(self.servers[sec%self.servers.len()].clone()).await?;

        // server.judger_info(request)

        todo!()
    }
}