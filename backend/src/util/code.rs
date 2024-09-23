use grpc::backend::StateCode as BackendCode;
use grpc::judger::JudgerCode;

/// Stabilized JudgeResponse Code, store in database
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum Code {
    Accepted = 1,
    WrongAnswer = 2,
    TimeLimitExceeded = 3,
    MemoryLimitExceeded = 4,
    RuntimeError = 5,
    CompileError = 6,
    SystemError = 7,
    RestrictedFunction = 8,
    Unknown = 9,
    OutputLimitExceeded = 10,
}

impl TryFrom<u32> for Code {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Code::Accepted),
            2 => Ok(Code::WrongAnswer),
            3 => Ok(Code::TimeLimitExceeded),
            4 => Ok(Code::MemoryLimitExceeded),
            5 => Ok(Code::RuntimeError),
            6 => Ok(Code::CompileError),
            7 => Ok(Code::SystemError),
            8 => Ok(Code::RestrictedFunction),
            9 => Ok(Code::Unknown),
            10 => Ok(Code::OutputLimitExceeded),
            _ => Err(()),
        }
    }
}

impl From<Code> for JudgerCode {
    fn from(value: Code) -> Self {
        match value {
            Code::Accepted => JudgerCode::Ac,
            Code::WrongAnswer => JudgerCode::Wa,
            Code::TimeLimitExceeded => JudgerCode::Tle,
            Code::MemoryLimitExceeded => JudgerCode::Mle,
            Code::RuntimeError => JudgerCode::Re,
            Code::CompileError => JudgerCode::Ce,
            Code::SystemError => JudgerCode::Na,
            Code::RestrictedFunction => JudgerCode::Rf,
            Code::Unknown => JudgerCode::Na,
            Code::OutputLimitExceeded => JudgerCode::Ole,
        }
    }
}

impl From<JudgerCode> for Code {
    fn from(value: JudgerCode) -> Self {
        match value {
            JudgerCode::Re => Code::RuntimeError,
            JudgerCode::Na => Code::Unknown,
            JudgerCode::Wa => Code::WrongAnswer,
            JudgerCode::Ce => Code::CompileError,
            JudgerCode::Ac => Code::Accepted,
            JudgerCode::Rf => Code::RestrictedFunction,
            JudgerCode::Tle => Code::TimeLimitExceeded,
            JudgerCode::Mle => Code::MemoryLimitExceeded,
            JudgerCode::Ole => Code::OutputLimitExceeded,
        }
    }
}

impl From<Code> for BackendCode {
    fn from(value: Code) -> Self {
        match value {
            Code::Accepted => BackendCode::Accepted,
            Code::WrongAnswer => BackendCode::WrongAnswer,
            Code::TimeLimitExceeded => BackendCode::TimeLimitExcess,
            Code::MemoryLimitExceeded => BackendCode::MemoryLimitExcess,
            Code::RuntimeError => BackendCode::RuntimeError,
            Code::CompileError => BackendCode::CompileError,
            Code::SystemError => BackendCode::Unknown,
            Code::RestrictedFunction => BackendCode::RestrictedFunction,
            Code::Unknown => BackendCode::Unknown,
            Code::OutputLimitExceeded => BackendCode::OutputLimitExcess,
        }
    }
}
impl From<BackendCode> for Code {
    fn from(value: BackendCode) -> Self {
        match value {
            BackendCode::Accepted => Code::Accepted,
            BackendCode::WrongAnswer => Code::WrongAnswer,
            BackendCode::TimeLimitExcess => Code::TimeLimitExceeded,
            BackendCode::MemoryLimitExcess => Code::MemoryLimitExceeded,
            BackendCode::RuntimeError => Code::RuntimeError,
            BackendCode::CompileError => Code::CompileError,
            BackendCode::Unknown => Code::SystemError,
            BackendCode::RestrictedFunction => Code::RestrictedFunction,
            BackendCode::OutputLimitExcess => Code::OutputLimitExceeded,
        }
    }
}
