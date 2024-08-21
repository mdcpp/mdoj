use super::{corpse::Corpse, error::Error, monitor::*, nsjail::*, Context, Filesystem};
use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
    process::Stdio,
};
use tokio::{
    io::{self, AsyncWriteExt, DuplexStream},
    process::*,
    time,
};
/// A not yet launched process that is mounted with a filesystem
struct MountedProcess<C: Context> {
    context: C,
    fs: C::FS,
}

impl<C: Context> MountedProcess<C> {
    fn new(mut context: C) -> Self {
        Self {
            fs: context.get_fs(),
            context,
        }
    }
}

/// a monitored process
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
        let output_limit = context.get_output();
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

/// A running process
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
    fn get_env(&mut self) -> OsString {
        let root = self.fs.get_path();
        // FIXME: check spec before unwrap
        let jail = self.context.get_args().next().unwrap();
        let real_path = PathBuf::from([root.as_ref().as_os_str(), jail].join(OsStr::new("")));

        let mut ancestors = real_path.ancestors();
        ancestors.next().unwrap();
        ancestors.next().unwrap().as_os_str().to_os_string()
    }
    /// spawn a raw process
    fn spawn_raw_process(&mut self) -> Result<Child, Error> {
        let mut cmd = Command::new(NSJAIL_PATH);
        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        #[cfg(not(debug_assertions))]
        cmd.stderr(Stdio::null());
        #[cfg(debug_assertions)]
        cmd.stderr(Stdio::inherit());
        cmd.env("PATH", self.get_env());

        let arg_factory = ArgFactory::default()
            .add(BaseArg)
            .add(CGroupVersionArg)
            .add(CGroupMountArg {
                cg_name: self.monitor.get_cg_path(),
            })
            .add(MountArg {
                rootfs: self.fs.get_path().as_ref(),
            })
            .add(InnerProcessArg {
                inner_args: self.context.get_args(),
            });

        let args = arg_factory.build();

        log::trace!("spawn process with args: {:?}", args);
        cmd.args(args);

        Ok(cmd.spawn()?)
    }
    /// spawn a process and wait for it to finish
    pub async fn wait(mut self, input: Vec<u8>) -> Result<Corpse, Error> {
        let mut process = self.spawn_raw_process()?;

        let mut stdin = process.stdin.take().unwrap();
        tokio::spawn(async move { stdin.write_all(&input).await });

        let stdout = process.stdout.take().unwrap();
        let io_proxy = tokio::spawn(async move {
            let mut stdout = stdout;
            if let Err(err) = io::copy(&mut stdout, &mut self.stdout).await {
                log::debug!("Fail forwarding buffer: {}", err);
            }
        });

        let mut monitor = self.monitor;
        let code = tokio::select! {
            _=monitor.wait_exhaust()=>None,
            x=process.wait()=>{
                time::sleep(time::Duration::from_millis(100)).await;
                Some(x?)
            }
        };
        // wait for the proxy to finish for full output
        // in case of OLE, the monitor will drop and the proxy will be cancelled(yield)
        io_proxy.await.unwrap();

        Ok(Corpse {
            code,
            reason: monitor.poll_exhaust(),
            stdout: monitor.take_buffer(),
            stat: monitor.stat().await,
        })
    }
}
