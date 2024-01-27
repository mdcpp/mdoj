use std::process::{self, Child, Command, Stdio};

use std::path::Path;

static JAEGER: &str = "./jaeger-all-in-one";
pub struct JaegerGuard {
    child: Child,
}

impl Drop for JaegerGuard {
    fn drop(&mut self) {
        self.child.kill().unwrap();
    }
}
pub fn jaeger() -> JaegerGuard {
    let path = Path::new(JAEGER);
    if !path.exists() {
        log::error!("{} not found, please download manualy", JAEGER);
        process::exit(1);
    }
    let mut cmd = Command::new(JAEGER);
    cmd.stdin(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdout(Stdio::piped());
    let child = cmd.spawn().unwrap();
    log::info!("jaeger is running, see http://localhost:16686/");

    JaegerGuard { child }
}

pub fn config() {
    log::warn!("TODO: add config check.");
}
