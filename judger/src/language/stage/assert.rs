use std::sync::Arc;

use crate::{
    language::spec::Spec,
    sandbox::{Corpse, MonitorKind},
};

pub enum AssertionMode {
    SkipSpace,
    SkipContinousSpace,
    Exact,
}

pub enum AssertResult {
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
pub struct AssertRunner {
    pub spec: Arc<Spec>,
    pub corpse: Corpse,
}

impl AssertRunner {
    pub fn new(spec: Arc<Spec>, corpse: Corpse) -> Self {
        Self { spec, corpse }
    }
    fn assert_output(&self, output: &[u8], mode: AssertionMode) -> AssertResult {
        todo!()
    }
    pub fn get_result(&self, output: &[u8], mode: AssertionMode) -> AssertResult {
        match self.corpse.status() {
            Ok(status) => match status.success() {
                true => self.assert_output(output, mode),
                false => AssertResult::WrongAnswer,
            },
            Err(reason) => match reason {
                MonitorKind::Cpu => AssertResult::TimeLimitExceeded,
                MonitorKind::Memory => AssertResult::MemoryLimitExceeded,
                MonitorKind::Output => AssertResult::OutputLimitExceeded,
                MonitorKind::Walltime => AssertResult::RealTimeLimitExceeded,
            },
        }
    }
}
