use super::super::Error;
use entity::problem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time;
use tonic::transport::{Channel, Uri};
use tonic::Streaming;

use crate::grpc::proto::prelude::{judger_client::JudgerClient, *};

pub struct JudgeRouter {
    servers: Vec<JudgerServer>,
    secquence: AtomicUsize,
}

#[derive(Clone)]
struct JudgerServer {
    uri: Arc<Uri>,
    connection: JudgerClient<Channel>,
}

impl JudgeRouter {
    pub async fn route(
        &self,
        problem: problem::Model,
        code: Vec<u8>,
        lang: String,
    ) -> Result<Streaming<JudgeResponse>, Error> {
        loop {
            match self.send(&problem, &code, &lang).await {
                Ok(x) => break Ok(x),
                Err(err) => match err.should_retry() {
                    false => break Err(err),
                    true => {
                        time::sleep(time::Duration::from_secs(1)).await;
                    }
                },
            }
        }
    }
    async fn send(
        &self,
        problem: &problem::Model,
        code: &Vec<u8>,
        lang: &String,
    ) -> Result<Streaming<JudgeResponse>, Error> {
        let secquence = self.secquence.fetch_add(1, Ordering::Relaxed);
        let tests = problem::Tests::from_raw(problem.tests.clone());

        let request = JudgeRequest {
            lang_uid: lang.clone(),
            code: code.clone(),
            memory: problem.memory,
            time: problem.time,
            rule: JudgeMatchRule::SkipSnl as i32,
            tests: tests
                .0
                .into_iter()
                .map(|(input, output)| TestIo { input, output })
                .collect(),
        };
        let mut server = self.servers[secquence % self.servers.len()].clone();

        let res = server.connection.judge(request).await.map_err(|err| {
            log::warn!("Error {} when connection to judge-{}", err, server.uri);
            Error::ShouldRetry
        })?;

        let (_, stream, _) = res.into_parts();

        Ok(stream)
    }
}
