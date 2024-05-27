//! collection of steps for judge and execute

mod compile;
mod judge;
mod run;
mod stream;

pub use compile::Compiler;
use grpc::{judger::JudgeMatchRule, judger::JudgerCode};
pub use run::Runner;

/// internal status code, use to decouple the grpc status code
///
/// Status code is commonly use in OJ, it include example such as: AC, WA...
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

/// internal assertion mode, use to decouple the grpc status code
///
/// Assertion mode reperesent the way to compare the output
#[derive(Clone, Copy)]
pub enum AssertionMode {
    /// Skip single space and newline
    ///
    /// `a b`, and `a\nb\n` are the same
    SkipSpace,
    /// Skip continous space and newline
    ///
    /// `a b`, `ab          ` and `ab` are the same
    SkipContinousSpace,
    /// Exact match
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

impl From<StatusCode> for JudgerCode {
    fn from(value: StatusCode) -> Self {
        match value {
            StatusCode::Accepted => Self::Ac,
            StatusCode::WrongAnswer => Self::Wa,
            StatusCode::RuntimeError => Self::Re,
            StatusCode::TimeLimitExceeded => Self::Tle,
            StatusCode::MemoryLimitExceeded => Self::Mle,
            StatusCode::OutputLimitExceeded => Self::Ole,
            StatusCode::RealTimeLimitExceeded => Self::Na,
            StatusCode::CompileError => Self::Ce,
            StatusCode::SystemError => Self::Na,
        }
    }
}
