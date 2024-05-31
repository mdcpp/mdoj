use std::sync::Arc;

use crate::{
    language::{
        spec::{self, Spec},
        ExecuteResult,
    },
    sandbox::{Corpse, MonitorKind},
};

use super::StatusCode;

/// Third stage of language processing, stream execution result
pub struct Streamer {
    spec: Arc<Spec>,
    corpse: Corpse,
}

impl Streamer {
    pub fn new(spec: Arc<Spec>, corpse: Corpse) -> Self {
        Self { spec, corpse }
    }
    pub fn get_code(&self) -> StatusCode {
        match self.corpse.status() {
            Ok(status) => match status.success() {
                true => StatusCode::Accepted,
                false => StatusCode::RuntimeError,
            },
            Err(reason) => match reason {
                MonitorKind::Cpu => StatusCode::TimeLimitExceeded,
                MonitorKind::Memory => StatusCode::MemoryLimitExceeded,
                MonitorKind::Output => StatusCode::OutputLimitExceeded,
                MonitorKind::Walltime => StatusCode::RealTimeLimitExceeded,
            },
        }
    }
    pub fn get_result(&self) -> ExecuteResult {
        let stat = self.corpse.stat();
        ExecuteResult {
            status: self.get_code(),
            time: stat.cpu.total,
            memory: stat.memory.total,
            output: self.corpse.stdout().to_vec(),
        }
    }
}
