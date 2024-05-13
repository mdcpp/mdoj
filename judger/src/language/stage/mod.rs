mod compile;
mod judge;
mod run;

pub use compile::Compiler;
use grpc::judger::JudgeMatchRule;
pub use judge::Judger;
pub use run::Runner;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    Accepted,
    WrongAnswer,
    RuntimeError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    OutputLimitExceeded,
    RealTimeLimitExceeded,
    CompileError,
    SystemError,
}

#[derive(Clone, Copy)]
pub enum AssertionMode {
    SkipSpace,
    SkipContinousSpace,
    Exact,
}

impl From<i32> for AssertionMode {
    fn from(value: i32) -> Self {
        let mode: JudgeMatchRule = value.try_into().unwrap_or_default();
        mode.into()
    }
}

impl From<JudgeMatchRule> for AssertionMode {
    fn from(rule: JudgeMatchRule) -> Self {
        match rule {
            JudgeMatchRule::ExactSame => AssertionMode::Exact,
            JudgeMatchRule::IgnoreSnl => AssertionMode::SkipSpace,
            JudgeMatchRule::SkipSnl => AssertionMode::SkipContinousSpace,
        }
    }
}
