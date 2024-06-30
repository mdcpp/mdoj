use grpc::judger::{
    exec_result as execute_response, ExecResult as ExecuteResponse, JudgeResponse, JudgerCode, Log,
};

use super::stage::{AssertionMode, StatusCode};

// FIXME: use derive_builder to remove boilerplate code

pub struct JudgeArgs {
    pub(super) mem: u64,
    pub(super) cpu: u64,
    pub(super) input: Vec<Vec<u8>>,
    pub(super) output: Vec<Vec<u8>>,
    pub(super) mode: AssertionMode,
    pub(super) source: Vec<u8>,
}

pub struct ExecuteArgs {
    pub(super) mem: u64,
    pub(super) cpu: u64,
    pub(super) input: Vec<u8>,
    pub(super) source: Vec<u8>,
}

pub struct JudgeResult {
    pub status: StatusCode,
    pub time: u64,
    pub memory: u64,
}

impl From<JudgeResult> for JudgeResponse {
    fn from(value: JudgeResult) -> Self {
        JudgeResponse {
            status: Into::<JudgerCode>::into(value.status) as i32,
            time: value.time,
            memory: value.memory,
            accuracy: 0, // FIXME: accuracy
        }
    }
}

pub struct ExecuteResult {
    pub status: StatusCode,
    pub time: u64,
    pub memory: u64,
    pub output: Vec<u8>,
}

impl From<ExecuteResult> for ExecuteResponse {
    fn from(value: ExecuteResult) -> Self {
        macro_rules! execute_log {
            ($msg:expr) => {
                execute_response::Result::Log(Log {
                    level: 4,
                    msg: $msg.to_string(),
                })
            };
        }
        let result = match value.status {
            StatusCode::Accepted => execute_response::Result::Output(value.output),
            StatusCode::WrongAnswer => execute_log!("Wrong Answer"),
            StatusCode::RuntimeError => {
                execute_log!("Runtime Error, maybe program return non-zero code")
            }
            StatusCode::TimeLimitExceeded | StatusCode::RealTimeLimitExceeded => {
                execute_log!("Time Limit Exceeded")
            }
            StatusCode::MemoryLimitExceeded => execute_log!("Memory Limit Exceeded"),
            StatusCode::OutputLimitExceeded => execute_log!("Output Limit Exceeded"),
            StatusCode::CompileError => execute_log!("Compile Error"),
            _ => execute_log!("System Error"),
        };
        ExecuteResponse {
            result: Some(result),
        }
    }
}

pub struct JudgeArgBuilder {
    mem: Option<u64>,
    cpu: Option<u64>,
    input: Option<Vec<Vec<u8>>>,
    output: Option<Vec<Vec<u8>>>,
    mode: Option<AssertionMode>,
    source: Option<Vec<u8>>,
}

impl JudgeArgBuilder {
    pub fn new() -> Self {
        Self {
            mem: None,
            cpu: None,
            input: None,
            output: None,
            mode: None,
            source: None,
        }
    }
    pub fn mem(mut self, mem: u64) -> Self {
        self.mem = Some(mem);
        self
    }
    pub fn cpu(mut self, cpu: u64) -> Self {
        self.cpu = Some(cpu);
        self
    }
    pub fn input(mut self, input: impl Iterator<Item = Vec<u8>>) -> Self {
        self.input = Some(input.collect());
        self
    }
    pub fn output(mut self, output: impl Iterator<Item = Vec<u8>>) -> Self {
        self.output = Some(output.collect());
        self
    }
    pub fn mode(mut self, mode: AssertionMode) -> Self {
        self.mode = Some(mode);
        self
    }
    pub fn source(mut self, source: Vec<u8>) -> Self {
        self.source = Some(source);
        self
    }
    pub fn build(self) -> JudgeArgs {
        JudgeArgs {
            mem: self.mem.expect("mem is not set"),
            cpu: self.cpu.expect("cpu is not set"),
            input: self.input.expect("input is not set"),
            output: self.output.expect("output is not set"),
            mode: self.mode.expect("mode is not set"),
            source: self.source.expect("source is not set"),
        }
    }
}

pub struct ExecuteArgBuilder {
    mem: Option<u64>,
    cpu: Option<u64>,
    input: Option<Vec<u8>>,
    source: Option<Vec<u8>>,
}

impl ExecuteArgBuilder {
    pub fn new() -> Self {
        Self {
            mem: None,
            cpu: None,
            input: None,
            source: None,
        }
    }
    pub fn mem(mut self, mem: u64) -> Self {
        self.mem = Some(mem);
        self
    }
    pub fn cpu(mut self, cpu: u64) -> Self {
        self.cpu = Some(cpu);
        self
    }
    pub fn input(mut self, input: Vec<u8>) -> Self {
        self.input = Some(input);
        self
    }
    pub fn source(mut self, source: Vec<u8>) -> Self {
        self.source = Some(source);
        self
    }
    pub fn build(self) -> ExecuteArgs {
        ExecuteArgs {
            mem: self.mem.expect("mem is not set"),
            cpu: self.cpu.expect("cpu is not set"),
            input: self.input.expect("input is not set"),
            source: self.source.expect("source is not set"),
        }
    }
}
