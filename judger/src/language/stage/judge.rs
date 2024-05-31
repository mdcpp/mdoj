use std::sync::Arc;

use crate::{
    language::{spec::Spec, JudgeResult},
    sandbox::{Corpse, MonitorKind, Stat},
};

use super::{AssertionMode, StatusCode};

/// The third stage of language processing, compare the output
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
        let input = self.corpse.stdout();
        match mode {
            AssertionMode::SkipSpace => {
                // skip space and newline, continous space and single space is consider different
                let output = output.iter().map(|x| match x {
                    b'\n' | b' ' => b' ',
                    x => *x,
                });
                let input = input.iter().map(|x| match x {
                    b'\n' | b' ' => b' ',
                    x => *x,
                });
                for (i, o) in input.zip(output) {
                    if i != o {
                        return StatusCode::WrongAnswer;
                    }
                }
            }
            AssertionMode::SkipContinousSpace => {
                // skip space and newline, continous space is consider same
                let output = output.iter().map(|x| match x {
                    b'\n' | b' ' => b' ',
                    x => *x,
                });
                let input = input.iter().map(|x| match x {
                    b'\n' | b' ' => b' ',
                    x => *x,
                });
                let mut output = output.peekable();
                let mut input = input.peekable();
                while let (Some(&i), Some(&o)) = (input.peek(), output.peek()) {
                    if i == b' ' {
                        while let Some(&x) = input.peek() {
                            if x != b' ' {
                                break;
                            }
                            input.next();
                        }
                        while let Some(&x) = output.peek() {
                            if x != b' ' {
                                break;
                            }
                            output.next();
                        }
                    } else if i != o {
                        return StatusCode::WrongAnswer;
                    } else {
                        input.next();
                        output.next();
                    }
                }
                if input.peek().is_some() || output.peek().is_some() {
                    return StatusCode::WrongAnswer;
                }
            }
            AssertionMode::Exact => {
                for (i, o) in input.iter().zip(output.iter()) {
                    if i != o {
                        return StatusCode::WrongAnswer;
                    }
                }
            }
        }

        StatusCode::Accepted
    }
    pub fn get_code(&self, output: &[u8], mode: AssertionMode) -> StatusCode {
        match self.corpse.status() {
            Ok(status) => match status.success() {
                true => self.assert_output(output, mode),
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
    pub fn get_result(&self, output: &[u8], mode: AssertionMode) -> JudgeResult {
        let status = self.get_code(output, mode);
        let stat = self.stat();
        JudgeResult {
            status,
            time: stat.cpu.total,
            memory: stat.memory.total,
        }
    }
}
