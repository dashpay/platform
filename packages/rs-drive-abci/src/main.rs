//! Main server process for RS-Drive-ABCI
//!
//! RS-Drive-ABCI server starts a single-threaded server and listens to connections from Tenderdash.

use clap::{Parser, Subcommand};
use drive_abci::config::{FromEnv, PlatformConfig};
use drive_abci::core::wait_for_core_to_sync;
use drive_abci::logging::{LogBuilder, LogConfig, Loggers};
use drive_abci::metrics::{Prometheus, DEFAULT_PROMETHEUS_PORT};
use drive_abci::rpc::core::DefaultCoreRPC;
use itertools::Itertools;
use std::path::PathBuf;
use tracing::warn;

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
    /// * none     - use RUST_LOG variable, default to `info`{n}
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

    /// Display colorful logs
    #[arg(long)]
    color: Option<bool>,
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

    /// Check status.
    ///
    /// Returns 0 on success.
    #[command()]
    Status {},
}

pub fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let config = load_config(&cli.config);

    configure_logging(&cli, &config).expect("failed to configure logging");

    install_panic_hook();

    match cli.command {
        Commands::Start {} => {
            let core_rpc = DefaultCoreRPC::open(
                config.core.rpc.url().as_str(),
                config.core.rpc.username.clone(),
                config.core.rpc.password.clone(),
            )
            .unwrap();

            let _prometheus = start_prometheus(&config)?;

            // Drive and Tenderdash rely on Core. Various functions will fail if Core is not synced.
            // We need to make sure that Core is ready before we start Drive ABCI app
            // Tenderdash won't start too until ABCI port is open.
            wait_for_core_to_sync(&core_rpc).unwrap();

            drive_abci::abci::start(&config, core_rpc).unwrap();
            Ok(())
        }
        Commands::Config {} => dump_config(&config),
        Commands::Status {} => check_status(&config),
    }
}

/// Start prometheus exporter if it's configured.
fn start_prometheus(config: &PlatformConfig) -> Result<Option<Prometheus>, String> {
    let prometheus_addr = config
        .abci
        .prometheus_bind_address
        .clone()
        .filter(|s| !s.is_empty());

    if let Some(addr) = prometheus_addr {
        let addr = url::Url::parse(&addr).map_err(|e| e.to_string())?;
        Ok(Some(
            drive_abci::metrics::Prometheus::new(addr).map_err(|e| e.to_string())?,
        ))
    } else {
        Ok(None)
    }
}

fn dump_config(config: &PlatformConfig) -> Result<(), String> {
    let serialized =
        serde_json::to_string_pretty(config).expect("failed to generate configuration");

    println!("{}", serialized);

    Ok(())
}

/// Check status of ABCI server.
fn check_status(config: &PlatformConfig) -> Result<(), String> {
    if let Some(prometheus_addr) = &config.abci.prometheus_bind_address {
        let url =
            url::Url::parse(prometheus_addr).expect("cannot parse ABCI_PROMETHEUS_BIND_ADDRESS");

        let addr = format!(
            "{}://{}:{}/metrics",
            url.scheme(),
            url.host()
                .ok_or("ABCI_PROMETHEUS_BIND_ADDRESS must contain valid host".to_string())?,
            url.port().unwrap_or(DEFAULT_PROMETHEUS_PORT)
        );

        let body: String = ureq::get(&addr)
            .set("Content-type", "text/plain")
            .call()
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())?;

        println!("{}", body);
        Ok(())
    } else {
        Err("ABCI_PROMETHEUS_BIND_ADDRESS not defined, cannot check status".to_string())
    }
}

fn load_config(path: &Option<PathBuf>) -> PlatformConfig {
    if let Some(path) = path {
        if let Err(e) = dotenvy::from_path(path) {
            panic!("cannot load config file {:?}: {}", path, e);
        }
    } else if let Err(e) = dotenvy::dotenv() {
        if e.not_found() {
            warn!("cannot find any matching .env file");
        } else {
            panic!("cannot load config file: {}", e);
        }
    }

    let config = PlatformConfig::from_env();
    if let Err(ref e) = config {
        if let drive_abci::error::Error::Configuration(envy::Error::MissingValue(field)) = e {
            panic!("missing configuration option: {}", field.to_uppercase());
        }
        panic!("cannot parse configuration file: {}", e);
    };

    config.expect("cannot parse configuration file")
}

fn configure_logging(
    cli: &Cli,
    config: &PlatformConfig,
) -> Result<Loggers, drive_abci::logging::Error> {
    let mut configs = config.abci.log.clone();
    if configs.is_empty() || cli.verbose > 0 {
        let cli_config = LogConfig {
            destination: "stderr".to_string(),
            verbosity: cli.verbose,
            color: cli.color,
            ..Default::default()
        };
        // we use key with underscores which are not allowed in config read from env
        configs.insert("cli_verbosity".to_string(), cli_config);
    }

    let loggers = LogBuilder::new().with_configs(&configs)?.build();
    loggers.install();

    tracing::info!("Configured log destinations: {}", configs.keys().join(","));

    Ok(loggers)
}

/// Install panic hook to ensure that all panic logs are correctly formatted.
///
/// Depends on [set_verbosity()].
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| tracing::error!(panic=%info, "panic")));
}
