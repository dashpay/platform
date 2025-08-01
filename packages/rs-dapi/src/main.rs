use clap::{ArgAction, Parser, Subcommand};
use rs_dapi::DAPIResult;
use std::path::PathBuf;
use std::process::ExitCode;
use tracing::{error, info, trace};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use rs_dapi::config::Config;
use rs_dapi::server::DapiServer;

#[derive(Debug, Subcommand)]
enum Commands {
    /// Start the DAPI server
    ///
    /// Starts all configured services including gRPC API, gRPC Streams,
    /// JSON-RPC, and optionally REST Gateway and Health Check endpoints.
    /// The server will run until interrupted with Ctrl+C.
    #[command()]
    Start,
    /// Display current configuration
    ///
    /// Shows all configuration variables and their current values from:
    /// 1. Environment variables
    /// 2. .env file (if specified or found)
    /// 3. Default values
    ///
    /// This is useful for debugging configuration issues and verifying
    /// which settings will be used.
    ///
    /// WARNING: Output may contain sensitive data like API keys or URIs!
    #[command()]
    Config,
    /// Print current software version
    ///
    /// Display the version information for rs-dapi and exit.
    #[command()]
    Version,
}

/// DAPI (Distributed API) server for Dash Platform
///
/// Provides gRPC, REST, and JSON-RPC endpoints for blockchain and platform data.
#[derive(Debug, Parser)]
#[command(
    name = "rs-dapi",
    version,
    about = "DAPI (Distributed API) server for Dash Platform",
    long_about = include_str!("../README.md")
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to the config (.env) file
    ///
    /// If not specified, rs-dapi will look for .env in the current directory.
    /// Variables in the environment always override .env file values.
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    config: Option<PathBuf>,

    /// Enable verbose logging. Use multiple times for even more logs
    ///
    /// Repeat 'v' multiple times to increase log verbosity:
    ///
    /// * none   - default to 'info' level for rs-dapi, 'warn' for libraries
    /// * -v     - 'debug' level for rs-dapi, 'info' for libraries  
    /// * -vv    - 'trace' level for rs-dapi, 'debug' for libraries
    /// * -vvv   - 'trace' level for all components
    ///
    /// Note: Using -v overrides any settings defined in RUST_LOG.
    #[arg(
        short = 'v',
        long = "verbose",
        action = ArgAction::Count,
        global = true
    )]
    verbose: u8,

    /// Display colorful logs
    ///
    /// Controls whether log output includes ANSI color codes.
    /// If not specified, color is automatically detected based on terminal capabilities.
    #[arg(long)]
    color: Option<bool>,

    /// Enable debug mode (equivalent to -vv)
    ///
    /// This is a convenience flag that sets the same log level as -vv:
    /// 'trace' level for rs-dapi, 'debug' level for libraries.
    #[arg(long)]
    debug: bool,
}

impl Cli {
    async fn run(self) -> Result<(), String> {
        // Load configuration
        let config = load_config(&self.config);

        // Configure logging
        configure_logging(&self)?;

        match self.command.unwrap_or(Commands::Start) {
            Commands::Start => {
                info!(
                    version = env!("CARGO_PKG_VERSION"),
                    rust = env!("CARGO_PKG_RUST_VERSION"),
                    "rs-dapi server initializing",
                );

                if let Err(e) = run_server(config).await {
                    error!("Server error: {}", e);
                    return Err(e.to_string());
                }
                Ok(())
            }
            Commands::Config => dump_config(&config),
            Commands::Version => {
                print_version();
                Ok(())
            }
        }
    }
}

fn load_config(path: &Option<PathBuf>) -> Config {
    match Config::load_from_dotenv(path.clone()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    }
}

fn configure_logging(cli: &Cli) -> Result<(), String> {
    // Determine log level based on verbose flags
    let env_filter = if cli.debug || cli.verbose > 0 {
        match cli.verbose.max(if cli.debug { 2 } else { 0 }) {
            1 => "rs_dapi=debug,info", // -v: debug from rs-dapi, info from others
            2 => "rs_dapi=trace,info", // -vv or --debug: trace from rs-dapi, debug from others
            3 => "rs_dapi=trace,h2=info,tower=info,hyper_util=info,debug", // -vvv
            4 => "rs_dapi=trace,debug", // -vvvv
            _ => "rs_dapi=trace,trace", // -vvvvv+
        }
    } else {
        // Use RUST_LOG if set, otherwise default
        &std::env::var("RUST_LOG").unwrap_or_else(|_| "rs_dapi=info,warn".to_string())
    };

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(env_filter))
        .map_err(|e| format!("Invalid log filter: {}", e))?;

    // Configure subscriber with color support
    let fmt_layer = tracing_subscriber::fmt::layer().with_ansi(cli.color.unwrap_or(true));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

async fn run_server(config: Config) -> DAPIResult<()> {
    trace!("Creating DAPI server instance...");
    let server = DapiServer::new(std::sync::Arc::new(config)).await?;

    info!("rs-dapi server starting on configured ports");

    trace!("Starting server main loop...");
    server.run().await?;

    info!("rs-dapi server shutdown complete");
    Ok(())
}

fn dump_config(config: &Config) -> Result<(), String> {
    println!("# rs-dapi Configuration");
    println!("# WARNING: This output may contain sensitive data!");
    println!();

    match serde_json::to_string_pretty(config) {
        Ok(json) => {
            println!("{}", json);
            Ok(())
        }
        Err(e) => Err(format!("Failed to serialize configuration: {}", e)),
    }
}

fn print_version() {
    println!("rs-dapi {}", env!("CARGO_PKG_VERSION"));
    println!("Built with Rust {}", env!("CARGO_PKG_RUST_VERSION"));
}

fn main() -> Result<(), ExitCode> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    let cli = Cli::parse();

    match rt.block_on(cli.run()) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            Err(ExitCode::FAILURE)
        }
    }
}
