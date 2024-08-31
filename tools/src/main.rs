mod grpc;
mod quoj;
mod quoj2mdoj;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
enum Cli {
    Quoj2mdoj(quoj2mdoj::Quoj2mdoj),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli {
        Cli::Quoj2mdoj(v) => quoj2mdoj::quoj2mdoj(v).await?,
    };

    Ok(())
}
