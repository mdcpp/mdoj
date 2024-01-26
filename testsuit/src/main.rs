pub mod client;
pub mod constant;
pub mod grpc;
pub mod macro_tool;
pub mod tests;

use clap::Parser;

static DATA_PATH:&str="data.toml";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// force restart
    #[arg(long, default_value_t = false)]
    force_restart: bool,
    /// whether to check backend/judger config
    #[arg(long, default_value_t = true)]
    check_config: bool,
    /// whether to run jaeger
    #[arg(long, default_value_t = true)]
    jaeger: bool,
}

#[async_std::main]
async fn main() {
    let state: tests::State = todo!();
    let logger = pretty_env_logger::formatted_builder().build();

    indicatif_log_bridge::LogWrapper::new(state.bar.clone(), logger)
        .try_init()
        .unwrap();

    let args = Args::parse();

    tests::run(state).await;
}
