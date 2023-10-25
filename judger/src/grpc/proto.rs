pub mod prelude {
    tonic::include_proto!("oj.judger");
}

use std::fmt::Display;

impl Display for prelude::JudgeResultState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            prelude::JudgeResultState::Ac => "Accepted",
            prelude::JudgeResultState::Na => "Unknown",
            prelude::JudgeResultState::Wa => "Wrong Answer",
            prelude::JudgeResultState::Ce => "Compile Error",
            prelude::JudgeResultState::Re => "Runtime Error",
            prelude::JudgeResultState::Rf => "Restricted Function",
            prelude::JudgeResultState::Tle => "Time Limit Excess",
            prelude::JudgeResultState::Mle => "Memory Limit Excess",
            prelude::JudgeResultState::Ole => "Output Limit Excess",
        };
        write!(f, "{}", message)
    }
}
