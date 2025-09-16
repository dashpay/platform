use clap::{ArgAction, Parser, Subcommand};
use rs_dapi::DAPIResult;
use rs_dapi::error::DapiError;
use std::path::PathBuf;
use std::process::ExitCode;
use tracing::{error, info, trace};

use rs_dapi::config::Config;
use rs_dapi::logging::{LoggingCliConfig, init_logging};
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

        // Configure logging and access logging
        let access_logger = configure_logging(&self, &config.dapi.logging).await?;

        match self.command.unwrap_or(Commands::Start) {
            Commands::Start => {
                info!(
                    version = env!("CARGO_PKG_VERSION"),
                    rust = env!("CARGO_PKG_RUST_VERSION"),
                    "rs-dapi server initializing",
                );

                if let Err(e) = run_server(config, access_logger).await {
                    error!("Server error: {}", e);

                    // Check if this is a connection-related error and set appropriate exit code
                    match &e {
                        DapiError::ServerUnavailable(_, _) => {
                            error!(
                                "Upstream service connection failed. Use --force to start without affected services."
                            );
                            return Err(format!("Connection error: {}", e));
                        }
                        DapiError::Client(msg) if msg.contains("Failed to connect") => {
                            error!(
                                "Client connection failed. Use --force to start without affected services."
                            );
                            return Err(format!("Connection error: {}", e));
                        }
                        DapiError::Transport(_) => {
                            error!(
                                "Transport error occurred. Use --force to start without affected services."
                            );
                            return Err(format!("Connection error: {}", e));
                        }
                        _ => return Err(e.to_string()),
                    }
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

async fn configure_logging(
    cli: &Cli,
    logging_config: &rs_dapi::config::LoggingConfig,
) -> Result<Option<rs_dapi::logging::AccessLogger>, String> {
    let cli_config = LoggingCliConfig {
        verbose: cli.verbose,
        debug: cli.debug,
        color: cli.color,
    };

    init_logging(logging_config, &cli_config).await
}

async fn run_server(
    config: Config,
    access_logger: Option<rs_dapi::logging::AccessLogger>,
) -> DAPIResult<()> {
    trace!("Creating DAPI server instance...");

    let server = DapiServer::new(std::sync::Arc::new(config), access_logger).await?;

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
