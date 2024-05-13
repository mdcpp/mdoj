use super::stage::{AssertionMode, StatusCode};

pub struct JudgeArgs {
    pub(super) mem: u64,
    pub(super) cpu: u64,
    pub(super) input: Vec<u8>,
    pub(super) output: Vec<u8>,
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

pub struct ExecuteResult {
    pub time: u64,
    pub memory: u64,
    pub output: Vec<u8>,
    pub code: i32,
}
