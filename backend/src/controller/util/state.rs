use tokio::sync::broadcast::*;

use crate::grpc::backend::{submit_status, SubmitStatus};
use crate::grpc::judger::{judge_response, JudgeResponse};

// impl Into<JudgeState> for JudgeResultState {
//     fn into(self) -> JudgeState {
//         match self {
//             JudgeResultState::Ac => JudgeState::Ac,
//             JudgeResultState::Wa => JudgeState::Wa,
//             JudgeResultState::Tle => JudgeState::Tle,
//             JudgeResultState::Mle => JudgeState::Mle,
//             JudgeResultState::Re => JudgeState::Re,
//             JudgeResultState::Ce => JudgeState::Ce,
//             JudgeResultState::Ole => JudgeState::Ole,
//             JudgeResultState::Na => JudgeState::Na,
//             JudgeResultState::Rf => JudgeState::Rf,
//         }
//     }
// }

// impl Into<JudgeResultState> for JudgeState {
//     fn into(self) -> JudgeResultState {
//         match self {
//             JudgeState::Ac => JudgeResultState::Ac,
//             JudgeState::Wa => JudgeResultState::Wa,
//             JudgeState::Tle => JudgeResultState::Tle,
//             JudgeState::Mle => JudgeResultState::Mle,
//             JudgeState::Re => JudgeResultState::Re,
//             JudgeState::Ce => JudgeResultState::Ce,
//             JudgeState::Ole => JudgeResultState::Ole,
//             JudgeState::Na => JudgeResultState::Na,
//             JudgeState::Rf => JudgeResultState::Rf,
//         }
//     }
// }

pub fn parse_state<M>(tx: &mut Sender<SubmitStatus>, res: JudgeResponse) {
    match res.task.unwrap_or_default() {
        judge_response::Task::Case(case) => {
            tx.send(SubmitStatus {
                task: Some(submit_status::Task::Case(case)),
            })
            .ok();
        }
        judge_response::Task::Result(res) => {
            todo!()
        }
    }
}
