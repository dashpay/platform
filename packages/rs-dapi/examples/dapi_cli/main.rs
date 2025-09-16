mod error;
mod masternode;
mod state_transition;
mod transactions;

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
    /// Stream transactions with proofs from the Core gRPC service
    Transactions(transactions::TransactionsCommand),
    /// Stream masternode list diffs from the Core gRPC service
    Masternode(masternode::MasternodeCommand),
    /// Platform state transition helpers
    #[command(subcommand_required = true)]
    StateTransition {
        #[command(subcommand)]
        command: state_transition::StateTransitionCommand,
    },
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
        Command::Transactions(cmd) => transactions::run(&cli.url, cmd).await?,
        Command::Masternode(cmd) => masternode::run(&cli.url, cmd).await?,
        Command::StateTransition { command } => state_transition::run(&cli.url, command).await?,
    }

    Ok(())
}
