pub mod prelude {
    tonic::include_proto!("oj.judger");
}

use std::fmt::Display;

impl Display for prelude::JudgerCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            prelude::JudgerCode::Ac => "Accepted",
            prelude::JudgerCode::Na => "Unknown",
            prelude::JudgerCode::Wa => "Wrong Answer",
            prelude::JudgerCode::Ce => "Compile Error",
            prelude::JudgerCode::Re => "Runtime Error",
            prelude::JudgerCode::Rf => "Restricted Function",
            prelude::JudgerCode::Tle => "Time Limit Excess",
            prelude::JudgerCode::Mle => "Memory Limit Excess",
            prelude::JudgerCode::Ole => "Output Limit Excess",
        };
        write!(f, "{}", message)
    }
}
