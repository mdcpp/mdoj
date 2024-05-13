mod compile;
mod judge;
mod run;

pub use compile::Compiler;
pub use judge::Judger;
pub use run::Runner;

pub enum StatusCode {
    Accept,
    WrongAnswer,
    RuntimeError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    OutputLimitExceeded,
    RealTimeLimitExceeded,
    CompileError,
    SystemError,
}

pub enum AssertionMode {
    SkipSpace,
    SkipContinousSpace,
    Exact,
}
