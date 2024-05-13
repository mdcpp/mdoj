use std::sync::Arc;

use crate::{
    language::spec::Spec,
    sandbox::{Corpse, MonitorKind, Stat},
};

use super::{AssertionMode, StatusCode};

pub struct Judger {
    spec: Arc<Spec>,
    corpse: Corpse,
}

impl Judger {
    pub fn new(spec: Arc<Spec>, corpse: Corpse) -> Self {
        Self { spec, corpse }
    }
    pub fn stat(&self) -> Stat {
        let stat = self.corpse.stat();
        self.spec.get_raw_stat(stat)
    }
    // pub fn stream_output(&self) -> Vec<u8> {
    //     self.corpse.stream_stdout()
    // }
    fn assert_output(&self, output: &[u8], mode: AssertionMode) -> StatusCode {
        todo!()
    }
    pub fn get_result(&self, output: &[u8], mode: AssertionMode) -> StatusCode {
        match self.corpse.status() {
            Ok(status) => match status.success() {
                true => self.assert_output(output, mode),
                false => StatusCode::WrongAnswer,
            },
            Err(reason) => match reason {
                MonitorKind::Cpu => StatusCode::TimeLimitExceeded,
                MonitorKind::Memory => StatusCode::MemoryLimitExceeded,
                MonitorKind::Output => StatusCode::OutputLimitExceeded,
                MonitorKind::Walltime => StatusCode::RealTimeLimitExceeded,
            },
        }
    }
}
