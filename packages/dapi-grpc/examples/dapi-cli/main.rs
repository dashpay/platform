mod core;
mod error;
mod platform;

use clap::{ArgAction, Parser, Subcommand};
use error::CliResult;

#[derive(Parser, Debug)]
#[command(
    name = "dapi-cli",
    version,
    about = "Interactive utilities for rs-dapi"
)]
struct Cli {
    /// DAPI gRPC endpoint (applies to all commands)
    #[arg(long, global = true, default_value = "http://127.0.0.1:3005")]
    url: String,

    /// Increase logging verbosity (-v for debug, -vv for trace)
    #[arg(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Core gRPC helpers
    #[command(subcommand)]
    Core(core::CoreCommand),
    /// Platform gRPC helpers
    #[command(subcommand)]
    Platform(platform::PlatformCommand),
}

fn init_tracing(verbosity: u8) {
    let level = match verbosity {
        0 => std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        1 => "debug".to_string(),
        _ => "trace".to_string(),
    };

    let _ = tracing_subscriber::fmt()
        .with_env_filter(level)
        .with_target(false)
        .try_init();
}

#[tokio::main]
async fn main() -> CliResult<()> {
    let cli = Cli::parse();

    init_tracing(cli.verbose);

    match cli.command {
        Command::Core(command) => core::run(&cli.url, command).await?,
        Command::Platform(command) => platform::run(&cli.url, command).await?,
    }

    Ok(())
}
