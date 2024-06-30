//! Provide ability to limit resource such as memory limit, cpu limit, walltime limit and output limit
mod hier;
mod mem_cpu;
mod output;
mod stat;
mod walltime;
mod wrapper;

use std::{fmt::Display, sync::atomic::AtomicUsize, time::Duration};

pub use stat::*;
use tokio::io::AsyncRead;

use hier::*;

use self::output::Output;

use super::Error;

lazy_static::lazy_static! {
    /// is cgroup v2 being used
    pub static ref CGROUP_V2:bool=hier::MONITER_KIND.heir().v2();
}

pub trait Monitor {
    type Resource;
    /// wait for exhuast of resource
    ///
    /// This function is cancel safe.
    async fn wait_exhaust(&mut self) -> MonitorKind {
        // those low level call is likely have event listener(like epoll)
        // monitor should use those listener to implement this function
        loop {
            if let Some(reason) = self.poll_exhaust() {
                return reason;
            }
            tokio::time::sleep(Duration::from_millis(12)).await;
        }
    }
    /// poll for exhuast of resource
    ///
    /// Implementor should do bith [`wait_exhaust`] and [`poll_exhaust`]
    /// for better performance.
    fn poll_exhaust(&mut self) -> Option<MonitorKind>;
    /// get the resource usage
    ///
    /// Please note that [`poll_exhaust`] might not be called before this function
    async fn stat(self) -> Self::Resource;
}

/// Exit reason of the process
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MonitorKind {
    Memory,
    Output,
    Walltime,
    Cpu,
}

impl Display for MonitorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Cpu => "cpu time",
                Self::Output => "output limit",
                Self::Walltime => "wall time",
                Self::Memory => "memory",
            }
        )
    }
}

/// a collection monitor
pub struct StatMonitor<P: AsyncRead + Unpin> {
    mem_cpu: mem_cpu::Monitor,
    output: output::Monitor<P>,
    walltime: walltime::Monitor,
}

impl<P: AsyncRead + Unpin> Monitor for StatMonitor<P> {
    type Resource = Stat;

    async fn wait_exhaust(&mut self) -> MonitorKind {
        tokio::select! {
            x = self.mem_cpu.wait_exhaust() => x,
            x = self.output.wait_exhaust() => x,
            x = self.walltime.wait_exhaust() => x,
        }
    }
    fn poll_exhaust(&mut self) -> Option<MonitorKind> {
        macro_rules! check_exhaust {
            ($f:ident) => {
                if let Some(reason) = self.$f.poll_exhaust() {
                    return Some(reason);
                }
            };
        }

        check_exhaust!(mem_cpu);
        check_exhaust!(output);
        check_exhaust!(walltime);

        None
    }

    async fn stat(self) -> Self::Resource {
        let (memory, cpu) = self.mem_cpu.stat().await;
        let output = self.output.stat().await;
        let walltime = self.walltime.stat().await;

        Stat {
            memory,
            cpu,
            output,
            walltime,
        }
    }
}

impl<P: AsyncRead + Unpin> StatMonitor<P> {
    pub fn get_cg_path(&self) -> &str {
        self.mem_cpu.get_cg_path()
    }
    pub fn take_buffer(&mut self) -> Vec<u8> {
        self.output.take_buffer()
    }
}

pub struct StatMonitorBuilder<P: AsyncRead + Unpin> {
    mem_cpu: Option<mem_cpu::Monitor>,
    output: Option<output::Monitor<P>>,
    walltime: Option<walltime::Monitor>,
}

impl<P: AsyncRead + Unpin> Default for StatMonitorBuilder<P> {
    fn default() -> Self {
        Self {
            mem_cpu: Default::default(),
            output: Default::default(),
            walltime: Default::default(),
        }
    }
}

impl<P: AsyncRead + Unpin> StatMonitorBuilder<P> {
    pub fn mem_cpu(mut self, mem_cpu: MemAndCpu) -> Result<Self, Error> {
        self.mem_cpu = Some(mem_cpu::Monitor::new(mem_cpu)?);
        Ok(self)
    }
    pub fn output(mut self, output: Output, stdout: P) -> Self {
        self.output = Some(output::Monitor::new(output, stdout));
        self
    }
    pub fn walltime(mut self, walltime: Duration) -> Self {
        self.walltime = Some(walltime::Monitor::new(walltime));
        self
    }
    pub fn build(self) -> Result<StatMonitor<P>, Error> {
        Ok(StatMonitor {
            mem_cpu: self
                .mem_cpu
                .expect("mem_cpu is required to be set, use mem_cpu method to set it"),
            output: self
                .output
                .expect("output is required to be set, use output method to set it"),
            walltime: self
                .walltime
                .expect("walltime is required to be set, use walltime method to set it"),
        })
    }
}
