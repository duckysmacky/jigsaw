mod cli;

use clap::Parser;

use cli::CliArgs;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let _args = CliArgs::parse();

    tracing::info!("Hello, world!");
}
