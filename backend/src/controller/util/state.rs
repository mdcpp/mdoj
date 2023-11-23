use std::cmp;

use tokio::sync::broadcast::*;

use crate::grpc::backend::{
    self, judge_result, submit_status, StateCode as BackendCode, SubmitStatus,
};
use crate::grpc::judger::{judge_response, JudgeResponse, JudgeResultState as JudgeCode};

impl Into<BackendCode> for JudgeCode {
    fn into(self) -> BackendCode {
        match self {
            JudgeCode::Ac => BackendCode::Ac,
            JudgeCode::Wa => BackendCode::Wa,
            JudgeCode::Tle => BackendCode::Tle,
            JudgeCode::Mle => BackendCode::Mle,
            JudgeCode::Re => BackendCode::Re,
            JudgeCode::Ce => BackendCode::Ce,
            JudgeCode::Ole => BackendCode::Ole,
            JudgeCode::Na => BackendCode::Na,
            JudgeCode::Rf => BackendCode::Rf,
        }
    }
}

impl Into<JudgeCode> for BackendCode {
    fn into(self) -> JudgeCode {
        match self {
            BackendCode::Ac => JudgeCode::Ac,
            BackendCode::Wa => JudgeCode::Wa,
            BackendCode::Tle => JudgeCode::Tle,
            BackendCode::Mle => JudgeCode::Mle,
            BackendCode::Re => JudgeCode::Re,
            BackendCode::Ce => JudgeCode::Ce,
            BackendCode::Ole => JudgeCode::Ole,
            BackendCode::Na => JudgeCode::Na,
            BackendCode::Rf => JudgeCode::Rf,
        }
    }
}

#[derive(Default)]
pub struct State {
    pub time: u64,
    pub mem: u64,
    pub pass: usize,
}

impl State {
    pub fn parse_state(
        &mut self,
        tx: &mut Sender<Result<SubmitStatus, tonic::Status>>,
        res: JudgeResponse,
    ) {
        if res.task.is_none() {
            log::warn!("mismatch proto(judger)");
            return;
        }
        match res.task.unwrap() {
            judge_response::Task::Case(case) => {
                tx.send(Ok(SubmitStatus {
                    task: Some(submit_status::Task::Case(case)),
                }))
                .ok();
            }
            judge_response::Task::Result(res) => {
                tx.send(Ok(SubmitStatus {
                    // TODO: rework the judger.proto
                    task: Some(submit_status::Task::Result(backend::JudgeResult {
                        info: Some(judge_result::Info::Committed(judge_result::Committed {
                            code: JudgeCode::from_i32(res.status).unwrap_or_default().into(),
                            accuracy: res.accuracy,
                            time: res.max_time,
                            memory: res.max_mem,
                        })),
                    })),
                }))
                .ok();
                self.time = cmp::max(self.time, res.max_time);
                self.mem = cmp::max(self.mem, res.max_mem);
                if res.status() == JudgeCode::Ac {
                    self.pass += 1;
                }
            }
        }
    }
}
