//! Main server process for RS-Drive-ABCI
//!
//! RS-Drive-ABCI server starts a single-threaded server and listens to connections from Tenderdash.
use clap::{Parser, Subcommand};
use drive_abci::config::{FromEnv, PlatformConfig};
use std::path::PathBuf;
use tracing::warn;
use tracing_subscriber::prelude::*;

// struct aaa {}

/// Server that accepts connections from Tenderdash, and
/// executes Dash Platform logic as part of the ABCI++ protocol.
///
/// Server configuration is based on environment variables that can be
/// set in the environment or saved in .env file.
#[derive(Debug, Parser)]
#[command(author, version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Path to the config (.env) file.
    #[arg(short, long, value_hint = clap::ValueHint::FilePath) ]
    config: Option<std::path::PathBuf>,
    /// Enable verbose logging. Use multiple times for even more logs.
    ///
    /// Repeat `v` multiple times to increase log verbosity:
    ///
    /// * none     - `warn` unless overriden by RUST_LOG variable{n}
    /// * `-v`     - `info` from Drive, `error` from libraries{n}
    /// * `-vv`    - `debug` from Drive, `info` from libraries{n}
    /// * `-vvv`   - `debug` from all components{n}
    /// * `-vvvv`  - `trace` from Drive, `debug` from libraries{n}
    /// * `-vvvvv` - `trace` from all components{n}
    ///
    /// Note: Using `-v` overrides any settings defined in RUST_LOG.
    ///
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Start server in foreground.
    #[command()]
    Start {},
    /// Dump configuration
    ///
    /// WARNING: output can contain sensitive data!
    #[command()]
    Config {},
}

pub fn main() {
    let cli = Cli::parse();
    let config = load_config(&cli.config);

    set_verbosity(&cli);
    install_panic_hook();

    match cli.command {
        Commands::Start {} => drive_abci::abci::start(&config).unwrap(),
        Commands::Config {} => dump_config(&config),
    }
}

fn dump_config(config: &PlatformConfig) {
    let serialized =
        serde_json::to_string_pretty(config).expect("failed to generate configuration");

    println!("{}", serialized);
}

fn load_config(config: &Option<PathBuf>) -> PlatformConfig {
    match config {
        Some(path) => {
            if let Err(e) = dotenvy::from_path(path) {
                panic!("cannot load config file {:?}: {}", path, e);
            }
        }
        None => {
            if let Err(e) = dotenvy::dotenv() {
                if e.not_found() {
                    warn!("cannot find any matching .env file");
                } else {
                    panic!("cannot load config file: {}", e);
                }
            }
        }
    };

    PlatformConfig::from_env().expect("cannot parse configuration file")
}

fn set_verbosity(cli: &Cli) {
    use tracing_subscriber::*;

    let env_filter = match cli.verbose {
        0 => EnvFilter::builder()
            .with_default_directive(
                "error,tenderdash_abci=warn,drive_abci=warn"
                    .parse()
                    .unwrap(),
            )
            .from_env_lossy(),
        1 => EnvFilter::new("error,tenderdash_abci=info,drive_abci=info"),
        2 => EnvFilter::new("info,tenderdash_abci=debug,drive_abci=debug"),
        3 => EnvFilter::new("debug,tenderdash_abci=debug,drive_abci=debug"),
        4 => EnvFilter::new("debug,tenderdash_abci=trace,drive_abci=trace"),
        5 => EnvFilter::new("trace"),
        _ => panic!("max verbosity level is 5"),
    };

    let layer = fmt::layer().with_ansi(atty::is(atty::Stream::Stdout));

    registry().with(layer).with(env_filter).init();
}

/// Install panic hook to ensure that all panic logs are correctly formatted
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| tracing::error!(panic=%info, "panic")));
}
