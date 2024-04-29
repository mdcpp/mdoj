use super::{monitor::*, Context};
use crate::sandbox::Filesystem;
use crate::Error;
use std::process::Stdio;
use tokio::{
    io::{self, AsyncWriteExt, DuplexStream},
    process::*,
    time,
};

use super::{corpse::Corpse, nsjail::*};

struct MountedProcess<C: Context> {
    context: C,
    fs: C::FS,
}

impl<C: Context> MountedProcess<C> {
    fn new(mut context: C) -> Self {
        Self {
            fs: context.create_fs(),
            context,
        }
    }
}

struct MonitoredProcess<C: Context> {
    fs: C::FS,
    context: C,
    monitor: StatMonitor<DuplexStream>,
    stdout: DuplexStream,
}

impl<C: Context> MonitoredProcess<C> {
    fn new(context: C) -> Result<Self, Error> {
        let process = MountedProcess::new(context);
        let mut context = process.context;

        let mem = context.get_memory();
        let cpu = context.get_cpu();
        let walltime = context.get_walltime();
        let output_limit = context.get_output_limit();
        let (fake_stdout, stdout) = io::duplex(1024);

        Ok(Self {
            monitor: StatMonitorBuilder::default()
                .mem_cpu((mem, cpu))?
                .walltime(walltime)
                .output(output_limit, fake_stdout)
                .build()
                .unwrap(),
            stdout,
            context,
            fs: process.fs,
        })
    }
}

impl<C: Context> From<MonitoredProcess<C>> for Process<C> {
    fn from(value: MonitoredProcess<C>) -> Self {
        Process {
            fs: value.fs,
            context: value.context,
            monitor: value.monitor,
            stdout: value.stdout,
        }
    }
}

pub struct Process<C: Context> {
    fs: C::FS,
    context: C,
    monitor: StatMonitor<DuplexStream>,
    stdout: DuplexStream,
}

impl<C: Context> Process<C> {
    pub fn new(context: C) -> Result<Self, Error> {
        MonitoredProcess::new(context).map(Into::into)
    }
    fn spawn_raw_process(&mut self) -> Result<Child, Error> {
        let mut cmd = Command::new(NSJAIL_PATH);
        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        let arg_factory = ArgFactory::default()
            .add(BaseArg)
            .add(CGroupVersionArg)
            .add(CGroupMountArg {
                cg_name: self.monitor.get_cg_path(),
            })
            .add(MountArg {
                rootfs: self.fs.mount().as_ref().as_os_str(),
            })
            .add(InnerProcessArg {
                inner_args: self.context.get_args(),
            });

        cmd.args(arg_factory.build());

        Ok(cmd.spawn()?)
    }
    pub async fn wait(mut self, input: Vec<u8>) -> Result<Corpse, Error> {
        let mut process = self.spawn_raw_process()?;

        let mut stdin = process.stdin.take().unwrap();
        tokio::spawn(async move { stdin.write_all(&input).await });

        let stdout = process.stdout.take().unwrap();
        tokio::spawn(async move {
            let mut stdout = stdout;
            if let Err(err) = io::copy(&mut stdout, &mut self.stdout).await {
                log::debug!("Fail forwarding buffer: {}", err);
            }
        });

        let mut monitor = self.monitor;
        let code = tokio::select! {
            _=monitor.wait_exhaust()=>{None},
            x=process.wait()=>{
                time::sleep(time::Duration::from_millis(100)).await;
                Some(x?)}
        };

        Ok(Corpse {
            code,
            reason: monitor.poll_exhaust(),
            stdout: monitor.take_buffer(),
            stat: monitor.stat().await,
        })
    }
}
