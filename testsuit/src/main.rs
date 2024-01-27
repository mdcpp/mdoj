pub mod checks;
pub mod client;
pub mod constants;
pub mod grpc;
pub mod macro_tool;
pub mod tests;

use clap::Parser;
use indicatif::ProgressBar;

/// testsuit for backend/judger
#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
struct Args {
    /// force restart
    #[arg(long, default_value_t = false)]
    force_restart: bool,
    /// check backend/judger config
    #[arg(long, default_value_t = true)]
    config: bool,
    /// run jaeger
    #[arg(long, default_value_t = false)]
    jaeger: bool,
}

#[async_std::main]
async fn main() {
    let state = tests::State::load().await;
    let logger = pretty_env_logger::formatted_builder().build();

    indicatif_log_bridge::LogWrapper::new(state.bar.clone(), logger)
        .try_init()
        .unwrap();

    let args = Args::parse();

    let pb = state.bar.add(ProgressBar::new(2));
    pb.set_message("checking config");

    if args.jaeger {
        pb.inc(1);
        checks::jaeger();
    }
    if args.config {
        pb.inc(1);
        checks::config();
    }

    let state = tests::run(state).await;

    state.save().await;
}
