//! Main server process for RS-Drive-ABCI
//!
//! RS-Drive-ABCI server starts a single-threaded server and listens to connections from Tenderdash.
use clap::{Parser, Subcommand};
use drive_abci::config::{FromEnv, PlatformConfig};

use drive_abci::metrics::DEFAULT_PROMETHEUS_PORT;
use drive_abci::rpc::core::DefaultCoreRPC;
use std::path::PathBuf;
use tracing::warn;
use tracing_log::LogTracer;
use tracing_subscriber::prelude::*;

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

    configure_logging(&cli);

    install_panic_hook();

    match cli.command {
        Commands::Start {} => {
            let core_rpc = DefaultCoreRPC::open(
                config.core.rpc.url().as_str(),
                config.core.rpc.username.clone(),
                config.core.rpc.password.clone(),
            )
            .unwrap();

            let _prometheus = if let Some(addr) = config.abci.prometheus_bind_address.clone() {
                let addr = url::Url::parse(&addr).map_err(|e| e.to_string())?;
                Some(drive_abci::metrics::Prometheus::new(addr).map_err(|e| e.to_string())?)
            } else {
                None
            };

            drive_abci::abci::start(&config, core_rpc).unwrap();
            Ok(())
        }
        Commands::Config {} => dump_config(&config),
        Commands::Status {} => check_status(&config),
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
    if let Some(addr) = config.abci.prometheus_bind_address.clone() {
        let url = url::Url::parse(&addr).expect("cannot parse ABCI_PROMETHEUS_BIND_ADDRESS");

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

fn configure_logging(cli: &Cli) {
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
        3 => EnvFilter::new("debug"),
        4 => EnvFilter::new("debug,tenderdash_abci=trace,drive_abci=trace"),
        5 => EnvFilter::new("trace"),
        _ => panic!("max verbosity level is 5"),
    };

    let ansi = cli.color.unwrap_or(atty::is(atty::Stream::Stdout));
    let layer = fmt::layer().with_ansi(ansi);

    registry().with(layer).with(env_filter).init();

    LogTracer::init().expect("cannot initialize LogTracer");
}

/// Install panic hook to ensure that all panic logs are correctly formatted.
///
/// Depends on [set_verbosity()].
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| tracing::error!(panic=%info, "panic")));
}
