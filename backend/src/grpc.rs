pub mod judger {
    tonic::include_proto!("oj.judger");
}
pub mod backend {
    tonic::include_proto!("oj.backend");
}

impl Default for self::judger::judge_response::Task {
    fn default() -> Self {
        log::warn!("judge_response::Task::default() is call becuase oj.judger is outdated");
        Self::Case(0)
    }
}
