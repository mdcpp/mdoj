use std::process::ExitStatus;

use super::monitor::{MonitorKind, Stat};

pub struct Corpse {
    /// exit code of signal
    pub(super) code: Option<ExitStatus>,
    /// exit reason reported by monitor
    pub(super) reason: Option<MonitorKind>,
    pub(super) stdout: Vec<u8>,
    pub(super) stat: Stat,
}

impl Corpse {
    /// get the exit status of the process
    ///
    /// if the process is killed by resource limit mechanism
    /// (monitor dropped), return the reason
    pub fn status(&self) -> Result<ExitStatus, MonitorKind> {
        if let Some(reason) = self.reason {
            Err(reason)
        } else {
            Ok(self.code.unwrap())
        }
    }
    /// get the stdout of the process
    ///
    /// If the process is killed by resource limit mechanism,
    /// the stdout may be incomplete(but ordered)
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }
    /// get the resource usage of the process
    pub fn stat(&self) -> &Stat {
        &self.stat
    }
}
