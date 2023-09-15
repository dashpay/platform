//! Main server process for RS-Drive-ABCI
//!
//! RS-Drive-ABCI server starts a single-threaded server and listens to connections from Tenderdash.

use clap::{Parser, Subcommand};
use drive_abci::config::{FromEnv, PlatformConfig};
use drive_abci::core::wait_for_core_to_sync::v0::wait_for_core_to_sync_v0;
use drive_abci::logging::{LogBuilder, LogConfig, Loggers};
use drive_abci::metrics::{Prometheus, DEFAULT_PROMETHEUS_PORT};
use drive_abci::rpc::core::DefaultCoreRPC;
use itertools::Itertools;
use std::path::PathBuf;
use std::process::ExitCode;
use tokio::runtime::Builder;
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::warn;

const SHUTDOWN_TIMEOUT_MILIS: u64 = 5000; // 5s; Docker defaults to 10s

#[derive(Debug, Subcommand)]
enum Commands {
    /// Start server in foreground.
    #[command()]
    Start,
    /// Dump configuration
    ///
    /// WARNING: output can contain sensitive data!
    #[command()]
    Config,

    /// Check status.
    ///
    /// Returns 0 on success.
    #[command()]
    Status,
}

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
    /// * none   - use RUST_LOG variable, default to `info`{n}
    /// * `-v`   - `debug` from Drive, `info` from libraries{n}
    /// * `-vv`  - `trace` from Drive, `debug` from libraries{n}
    /// * `-vvv` - `trace` from all components{n}
    ///
    /// Note: Using `-v` overrides any settings defined in RUST_LOG.
    ///
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Display colorful logs
    #[arg(long)]
    color: Option<bool>,
}

impl Cli {
    fn run(self, config: PlatformConfig, cancel: CancellationToken) -> Result<(), String> {
        match self.command {
            Commands::Start => {
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
                wait_for_core_to_sync_v0(&core_rpc, cancel.clone()).unwrap();

                drive_abci::abci::start(&config, core_rpc, cancel).map_err(|e| e.to_string())?;
                return Ok(());
            }
            Commands::Config => dump_config(&config)?,
            Commands::Status => check_status(&config)?,
        };

        Ok(())
    }
}

fn main() -> Result<(), ExitCode> {
    let cli = Cli::parse();
    let config = load_config(&cli.config);
    // We use `cancel` to notify other subsystems that the server is shutting down
    let cancel = tokio_util::sync::CancellationToken::new();

    let loggers = configure_logging(&cli, &config).expect("failed to configure logging");

    install_panic_hook(cancel.clone());

    // Start tokio runtime and thread listening for signals.
    // The runtime will be reused by Prometheus and rs-tenderdash-abci.
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("cannot initialize tokio runtime");
    let rt_guard = runtime.enter();

    runtime.spawn(handle_signals(cancel.clone(), loggers));

    // Main thread is not started in runtime, as it is synchronous and we don't want to run into
    // potential, hard to debug, issues.
    let status = match cli.run(config, cancel) {
        Ok(()) => {
            tracing::debug!("shutdown complete");
            ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::error!(error = e, "drive-abci failed");
            ExitCode::FAILURE
        }
    };

    drop(rt_guard);
    runtime.shutdown_timeout(Duration::from_millis(SHUTDOWN_TIMEOUT_MILIS));
    tracing::info!("drive-abci server is down");

    Err(status)
}

/// Handle signals received from operating system
async fn handle_signals(cancel: CancellationToken, logs: Loggers) -> Result<(), String> {
    let mut sigint = signal(SignalKind::interrupt()).map_err(|e| e.to_string())?;
    let mut sigterm = signal(SignalKind::terminate()).map_err(|e| e.to_string())?;
    let mut sighup = signal(SignalKind::hangup()).map_err(|e| e.to_string())?;

    while !cancel.is_cancelled() {
        tokio::select! {
          _ = sigint.recv() => {
                tracing::info!("received SIGINT (ctrl+c), initiating shutdown");
                cancel.cancel();
            },
          _ = sigterm.recv() => {
                tracing::info!("received SIGTERM, initiating shutdown");
                cancel.cancel();
            },
        _ = sighup.recv() => {
                tracing::info!("received SIGHUP, flushing and rotating logs");
                if let Err(error) = logs.flush() {
                    tracing::error!(?error, "logs flush failed");
                };
                if let Err(error) = logs.rotate() {
                    tracing::error!(?error, "logs rotate failed");
                };
            },
          _ = cancel.cancelled() => tracing::trace!("shutting down signal handlers"),
        };
    }

    Ok(())
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
            level: cli.verbose.try_into().unwrap(),
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
/// Should be called after [set_verbosity()].
fn install_panic_hook(cancel: CancellationToken) {
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic=%info, "panic");
        cancel.cancel();
    }));
}
