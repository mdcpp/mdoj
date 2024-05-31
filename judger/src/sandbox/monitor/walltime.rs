use tokio::time::*;

use super::*;

pub type WallTime = Duration;

pub struct Monitor {
    dur: Duration,
    start: Option<Instant>,
}

impl Monitor {
    pub fn new(dur: Duration) -> Self {
        Self { dur, start: None }
    }
}

impl super::Monitor for Monitor {
    type Resource = WallTime;

    async fn wait_exhaust(&mut self) -> MonitorKind {
        self.start = Some(Instant::now());
        sleep(self.dur).await;
        MonitorKind::Walltime
    }
    fn poll_exhaust(&mut self) -> Option<MonitorKind> {
        if let Some(start) = self.start {
            if Instant::now() < start + self.dur {
                return None;
            }
        }
        Some(MonitorKind::Walltime)
    }
    async fn stat(self) -> Self::Resource {
        match self.start {
            Some(start) => Instant::now().duration_since(start),
            None => Duration::ZERO,
        }
    }
}
