pub mod checks;
pub mod client;
pub mod constants;
pub mod grpc;
pub mod macro_tool;
pub mod tests;

use std::time::Duration;

use clap::Parser;
use indicatif::ProgressBar;

use async_std::{
    io::{self, ReadExt},
    task::sleep,
};

/// testsuit for backend/judger
#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
struct Args {
    /// force restart
    #[arg(long, default_value_t = false)]
    force_restart: bool,
    /// check backend/judger config
    #[arg(long, default_value_t = false)]
    config: bool,
    /// run jaeger
    #[arg(long, default_value_t = true)]
    jaeger: bool,
}

#[async_std::main]
async fn main() {
    let args = Args::parse();

    let mut state = tests::State::load().await;
    if args.force_restart {
        state.step = 0;
    }

    let logger = pretty_env_logger::formatted_builder().build();

    indicatif_log_bridge::LogWrapper::new(state.bar.clone(), logger)
        .try_init()
        .unwrap();

    let pb = state.bar.add(ProgressBar::new(2));
    pb.set_message("checking config");

    let jaeger = if args.jaeger {
        pb.inc(1);
        Some(checks::jaeger())
    } else {
        None
    };
    if args.config {
        pb.inc(1);
        checks::config();
    }
    pb.finish_and_clear();

    let state = tests::run(state).await;

    state.save().await;
    if args.jaeger {
        let mut stdin = io::stdin();
        log::info!("Please check telemetry or enter anything to continue...");
        sleep(Duration::from_secs(1)).await;
        let _ = stdin.read(&mut [0u8]).await.unwrap();
    }
    drop(jaeger)
}
