use tokio::sync::broadcast::*;

use crate::grpc::backend::{self, submit_status, JudgeResultState as BackendState, SubmitStatus};
use crate::grpc::judger::{judge_response, JudgeResponse, JudgeResultState as JudgeState};

impl Into<BackendState> for JudgeState {
    fn into(self) -> BackendState {
        match self {
            JudgeState::Ac => BackendState::Ac,
            JudgeState::Wa => BackendState::Wa,
            JudgeState::Tle => BackendState::Tle,
            JudgeState::Mle => BackendState::Mle,
            JudgeState::Re => BackendState::Re,
            JudgeState::Ce => BackendState::Ce,
            JudgeState::Ole => BackendState::Ole,
            JudgeState::Na => BackendState::Na,
            JudgeState::Rf => BackendState::Rf,
        }
    }
}

impl Into<JudgeState> for BackendState {
    fn into(self) -> JudgeState {
        match self {
            BackendState::Ac => JudgeState::Ac,
            BackendState::Wa => JudgeState::Wa,
            BackendState::Tle => JudgeState::Tle,
            BackendState::Mle => JudgeState::Mle,
            BackendState::Re => JudgeState::Re,
            BackendState::Ce => JudgeState::Ce,
            BackendState::Ole => JudgeState::Ole,
            BackendState::Na => JudgeState::Na,
            BackendState::Rf => JudgeState::Rf,
        }
    }
}

pub fn parse_state(tx: &mut Sender<SubmitStatus>, res: JudgeResponse) {
    match res.task.unwrap_or_default() {
        judge_response::Task::Case(case) => {
            tx.send(SubmitStatus {
                task: Some(submit_status::Task::Case(case)),
            })
            .ok();
        }
        judge_response::Task::Result(res) => {
            tx.send(SubmitStatus {
                task: Some(submit_status::Task::Result(backend::JudgeResult {
                    status: res.status() as i32,
                    max_time: Some(res.max_time()),
                    max_mem: Some(res.max_mem()),
                })),
            })
            .ok();
        }
    }
}
