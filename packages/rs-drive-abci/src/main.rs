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
use std::process::ExitCode;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::warn;

const SHUTDOWN_TIMEOUT_MILIS: u64 = 3000;

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

fn main() -> Result<(), ExitCode> {
    let cli = Cli::parse();
    let config = load_config(&cli.config);

    configure_logging(&cli, &config).expect("failed to configure logging");

    install_panic_hook();

    // We configure runtime manually to have access to runtime::shutdown
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("cannot start tokio runtime");
    let rt_guard = runtime.enter();

    let status = match runtime.block_on(tokio_main(config, cli)) {
        Ok(()) => {
            tracing::debug!("shutdown complete");
            ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::error!(errors = e, "execution failed");
            ExitCode::FAILURE
        }
    };

    drop(rt_guard);
    runtime.shutdown_background();
    tracing::info!("drive-abci server is down");

    Err(status)
}

async fn tokio_main(config: PlatformConfig, cli: Cli) -> Result<(), String> {
    // We use `cancel` to notify other subsystems that the server is shutting down
    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_guard = cancel.clone().drop_guard();

    let mut threads: Vec<JoinHandle<Result<(), String>>> = Vec::new();
    threads.push(tokio::spawn(main_thread(cancel, config, cli)));

    handle_signals().await.map_err(|e| e.to_string())?;

    tracing::debug!("initiating graceful shutdown");
    drop(cancel_guard);

    shutdown_threads(threads).await
}

async fn handle_signals() -> Result<(), std::io::Error> {
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigquit = signal(SignalKind::quit())?;

    tokio::select! {
      _ = sigint.recv() => (),
      _ = sigterm.recv() => (),
      _ = sigquit.recv() => (),
    };

    Ok(())
}

async fn shutdown_threads(threads: Vec<JoinHandle<Result<(), String>>>) -> Result<(), String> {
    let end = tokio::time::Instant::now() + Duration::from_millis(SHUTDOWN_TIMEOUT_MILIS);
    while end.elapsed() == Duration::ZERO {
        if threads.iter().all(|hdl| hdl.is_finished()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let mut errors = String::new();
    // Here we either finished, or timed out
    for hdl in threads.into_iter() {
        if !hdl.is_finished() {
            hdl.abort();
            errors += &format!("shutdown timeout: {:?}\n", hdl);

            continue;
        }

        let res = hdl.await;
        let result = match res {
            Err(e) => e.to_string(),
            Ok(Err(e)) => e,
            Ok(Ok(())) => String::new(),
        };
        if !result.is_empty() {
            errors += &result;
            errors += "\n";
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

async fn main_thread(
    cancel: CancellationToken,
    config: PlatformConfig,
    cli: Cli,
) -> Result<(), String> {
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

            drive_abci::abci::start(cancel, &config, core_rpc).map_err(|e| e.to_string())?;
            return Ok(());
        }
        Commands::Config {} => dump_config(&config)?,
        Commands::Status {} => check_status(&config)?,
    };

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
