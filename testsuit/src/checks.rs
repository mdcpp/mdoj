use std::process::{self, Command, Stdio};

use std::path::Path;

static JAEGER: &str = "./jaeger-all-in-one";
pub fn jaeger() {
    let path = Path::new(JAEGER);
    if !path.exists() {
        log::error!("{} not found", JAEGER);
        process::exit(1);
    }
    std::thread::spawn(|| {
        let mut cmd = Command::new(JAEGER);
        cmd.stdin(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.spawn().unwrap();
        log::info!("jaeger is running, see http://localhost:16686/");
    });
}
pub fn config() {}
