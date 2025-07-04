//! Main server process for RS-Drive-ABCI
//!
//! RS-Drive-ABCI server starts a single-threaded server and listens to connections from Tenderdash.

use clap::{Parser, Subcommand};
use dapi_grpc::platform::v0::get_status_request;
use dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0;
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::tonic::transport::Uri;
use dpp::version::PlatformVersion;
use drive_abci::config::{FromEnv, PlatformConfig};
use drive_abci::core::wait_for_core_to_sync::v0::wait_for_core_to_sync_v0;
use drive_abci::logging::{LogBuilder, LogConfig, LogDestination, Loggers};
use drive_abci::metrics::Prometheus;
use drive_abci::platform_types::platform::Platform;
use drive_abci::rpc::core::DefaultCoreRPC;
use drive_abci::{logging, server};
use itertools::Itertools;
use std::fs::remove_file;
#[cfg(all(tokio_unstable, feature = "console"))]
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime::{Builder, Runtime};
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;
#[cfg(all(tokio_unstable, feature = "console"))]
use tracing_subscriber::layer::SubscriberExt;
#[cfg(all(tokio_unstable, feature = "console"))]
use tracing_subscriber::util::SubscriberInitExt;

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

    /// Verify integrity of database.
    ///
    /// This command will execute GroveDB hash integrity checks.
    ///
    /// You can also enforce grovedb integrity checks during `drive-abci start`
    /// by creating `.fsck` file in database directory (`DB_PATH`).
    #[command()]
    Verify,

    /// Print current software version
    #[command()]
    Version,
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
    config: Option<PathBuf>,

    /// Enable verbose logging. Use multiple times for even more logs.
    ///
    /// Repeat `v` multiple times to increase log verbosity:
    ///
    /// * none   -  default to `info`{n}
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
    fn run(
        self,
        runtime: &Runtime,
        config: PlatformConfig,
        cancel: CancellationToken,
    ) -> Result<(), String> {
        match self.command {
            Commands::Start => {
                tracing::info!(
                    version = env!("CARGO_PKG_VERSION"),
                    features = list_enabled_features().join(","),
                    rust = env!("CARGO_PKG_RUST_VERSION"),
                    "drive-abci server initializing",
                );

                if config.drive.grovedb_verify_on_startup {
                    verify_grovedb(&config.db_path, false)?;
                }
                let core_rpc = DefaultCoreRPC::open(
                    config.core.consensus_rpc.url().as_str(),
                    config.core.consensus_rpc.username.clone(),
                    config.core.consensus_rpc.password.clone(),
                )
                .unwrap();

                let _prometheus = start_prometheus(&config)?;

                // Drive and Tenderdash rely on Core. Various functions will fail if Core is not synced.
                // We need to make sure that Core is ready before we start Drive ABCI app
                // Tenderdash won't start too until ABCI port is open.
                wait_for_core_to_sync_v0(&core_rpc, cancel.clone()).map_err(|e| e.to_string())?;

                if cancel.is_cancelled() {
                    return Ok(());
                }

                let platform: Platform<DefaultCoreRPC> = Platform::open_with_client(
                    config.db_path.clone(),
                    Some(config.clone()),
                    core_rpc,
                    None,
                )
                .expect("Failed to open platform");

                server::start(runtime, Arc::new(platform), config, cancel);

                tracing::info!("drive-abci server is stopped");

                return Ok(());
            }
            Commands::Config => dump_config(&config)?,
            Commands::Status => runtime.block_on(check_status(&config))?,
            Commands::Verify => verify_grovedb(&config.db_path, true)?,
            Commands::Version => print_version(),
        };

        Ok(())
    }
}

fn main() -> Result<(), ExitCode> {
    let cli = Cli::parse();
    let config = load_config(&cli.config);

    // Start tokio runtime and thread listening for signals.
    // The runtime will be reused by Prometheus and rs-tenderdash-abci.
    let runtime = Builder::new_multi_thread()
        // TODO: 8 MB stack threads as some recursions in GroveDB can be pretty deep
        //  We could remove such a stack stack size once deletion of a node doesn't recurse in grovedb
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()
        .expect("cannot initialize tokio runtime");

    // We use `cancel` to notify other subsystems that the server is shutting down
    let cancel = CancellationToken::new();

    let loggers = configure_logging(&cli, &config).expect("failed to configure logging");

    // If tokio console is enabled, we install loggers together with tokio console
    // due to type compatibility issue

    #[cfg(not(feature = "console"))]
    loggers.install();

    #[cfg(feature = "console")]
    if config.tokio_console_enabled {
        #[cfg(not(tokio_unstable))]
        panic!("tokio_unstable flag should be set");

        // Initialize Tokio console subscriber
        #[cfg(tokio_unstable)]
        {
            let socket_addr: SocketAddr = config
                .tokio_console_address
                .parse()
                .expect("cannot parse tokio console address");

            let console_layer = console_subscriber::ConsoleLayer::builder()
                .retention(Duration::from_secs(config.tokio_console_retention_secs))
                .server_addr(socket_addr)
                .spawn();

            tracing_subscriber::registry()
                .with(
                    loggers
                        .tracing_subscriber_layers()
                        .expect("should return layers"),
                )
                .with(console_layer)
                .try_init()
                .expect("can't init tracing subscribers");
        }
    } else {
        loggers.install();
    }

    // Log panics

    install_panic_hook(cancel.clone());

    // Start runtime in the main thread
    let runtime_guard = runtime.enter();

    runtime.spawn(handle_signals(cancel.clone(), loggers));

    let result = cli.run(&runtime, config, cancel).map_err(|e| {
        tracing::error!(error = e, "drive-abci failed: {e}");

        ExitCode::FAILURE
    });

    drop(runtime_guard);
    runtime.shutdown_timeout(Duration::from_millis(SHUTDOWN_TIMEOUT_MILIS));
    result
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
        }
    }

    Ok(())
}

/// Start prometheus exporter if it's configured.
fn start_prometheus(config: &PlatformConfig) -> Result<Option<Prometheus>, String> {
    let prometheus_addr = config
        .prometheus_bind_address
        .clone()
        .filter(|s| !s.is_empty());

    if let Some(addr) = prometheus_addr {
        tracing::info!("Expose prometheus metrics on {}", addr);

        let addr = url::Url::parse(&addr).map_err(|e| e.to_string())?;
        Ok(Some(Prometheus::new(addr).map_err(|e| e.to_string())?))
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

fn list_enabled_features() -> Vec<&'static str> {
    vec![
        #[cfg(feature = "console")]
        "console",
        #[cfg(feature = "testing-config")]
        "testing-config",
        #[cfg(feature = "grovedbg")]
        "grovedbg",
        #[cfg(feature = "mocks")]
        "mocks",
    ]
}

/// Check status of ABCI server.
async fn check_status(config: &PlatformConfig) -> Result<(), String> {
    // Convert the gRPC bind address string to a Uri
    let uri = Uri::from_str(&format!("http://{}", config.grpc_bind_address))
        .map_err(|e| format!("invalid url: {e}"))?;

    // Connect to the gRPC server
    let mut client = PlatformClient::connect(uri.clone())
        .await
        .map_err(|e| format!("can't connect to grpc server {uri}: {e}"))?;

    // Make a request to the server
    let request = dapi_grpc::platform::v0::GetStatusRequest {
        version: Some(get_status_request::Version::V0(GetStatusRequestV0 {})),
    };

    // Should return non-zero error code if Drive is not responding
    client
        .get_status(request)
        .await
        .map(|_| ())
        .map_err(|e| format!("can't request status: {e}"))
}

/// Verify GroveDB integrity.
///
/// This function will execute GroveDB integrity checks if one of the following conditions is met:
/// - `force` is `true`
/// - file `.fsck` in `config.db_path` exists
///
/// After successful verification, .fsck file is removed.
fn verify_grovedb(db_path: &PathBuf, force: bool) -> Result<(), String> {
    let fsck = PathBuf::from(db_path).join(".fsck");

    if !force {
        if !fsck.exists() {
            return Ok(());
        }
        tracing::info!(
            "found {} file, starting grovedb verification",
            fsck.display()
        );
    }

    let grovedb = drive::grovedb::GroveDb::open(db_path).expect("open grovedb");
    //todo: get platform version instead of taking latest
    let result = grovedb
        .visualize_verify_grovedb(
            None,
            true,
            true,
            &PlatformVersion::latest().drive.grove_version,
        )
        .map_err(|e| e.to_string());

    match result {
        Ok(data) => {
            for result in data {
                tracing::warn!(?result, "grovedb verification")
            }
            tracing::info!("grovedb verification finished");

            if fsck.exists() {
                if let Err(e) = remove_file(&fsck) {
                    tracing::warn!(
                        error = ?e,
                        path  =fsck.display().to_string(),
                        "grovedb verification: cannot remove .fsck file: please remove it manually to avoid running verification again",
                    );
                }
            }
            Ok(())
        }
        Err(e) => {
            tracing::error!("grovedb verification failed: {}", e);
            Err(e)
        }
    }
}

/// Print current software version.
fn print_version() {
    println!("{}", env!("CARGO_PKG_VERSION"));
}

fn load_config(path: &Option<PathBuf>) -> PlatformConfig {
    if let Some(path) = path {
        if let Err(e) = dotenvy::from_path(path) {
            panic!("cannot load config file {:?}: {}", path, e);
        }
    } else if let Err(e) = dotenvy::dotenv() {
        if e.not_found() {
            tracing::warn!("cannot find any matching .env file");
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

fn configure_logging(cli: &Cli, config: &PlatformConfig) -> Result<Loggers, logging::Error> {
    let mut configs = config.abci.log.clone();
    if configs.is_empty() || cli.verbose > 0 {
        let cli_config = LogConfig {
            destination: LogDestination::StdOut,
            level: cli.verbose.try_into()?,
            color: cli.color,
            ..Default::default()
        };
        // we use key with underscores which are not allowed in config read from env
        configs.insert("cli_verbosity".to_string(), cli_config);
    }

    let loggers = LogBuilder::new().with_configs(&configs)?.build();

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

#[cfg(test)]
mod test {
    use ::drive::{drive::Drive, query::Element};
    use dpp::block::epoch::Epoch;
    use drive::drive::credit_pools::epochs::epoch_key_constants;
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use dpp::version::PlatformVersion;
    use drive::drive::credit_pools::epochs::paths::EpochProposers;
    use drive_abci::logging::LogLevel;
    use rocksdb::{IteratorMode, Options};

    /// Setup drive database by creating initial state structure and inserting some data.
    ///
    /// Returns path to the database.
    fn setup_db(tempdir: &Path) -> PathBuf {
        let path = tempdir.join("db");
        fs::create_dir(&path).expect("create db dir");

        let platform_version = PlatformVersion::latest();

        let (drive, _) = Drive::open(&path, None).expect("open drive");

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("should create root tree successfully");

        let transaction = drive.grove.start_transaction();
        let epoch = Epoch::new(0).unwrap();

        let i = 100;

        drive
            .grove
            .insert(
                &epoch.get_path(),
                epoch_key_constants::KEY_FEE_MULTIPLIER.as_slice(),
                Element::Item((i as u128).to_be_bytes().to_vec(), None),
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("should insert data");

        transaction.commit().unwrap();

        path
    }

    /// Open RocksDB and corrupt `n`-th item from `cf` column family.
    fn corrupt_rocksdb_item(db_path: &PathBuf, cf: &str, n: usize) {
        let mut db_opts = Options::default();

        db_opts.create_missing_column_families(false);
        db_opts.create_if_missing(false);

        let db = rocksdb::DB::open_cf(&db_opts, db_path, vec!["roots", "meta", "aux"]).unwrap();

        let cf_handle = db.cf_handle(cf).unwrap();
        let iter = db.iterator_cf(cf_handle, IteratorMode::Start);

        for (i, item) in iter.enumerate() {
            let (key, mut value) = item.unwrap();
            // println!("{} = {}", hex::encode(&key), hex::encode(&value));
            tracing::trace!(cf, key=?hex::encode(&key), value=hex::encode(&value),"found item in rocksdb");

            if i == n {
                value[0] = !value[0];
                db.put_cf(cf_handle, &key, &value).unwrap();

                tracing::debug!(cf, key=?hex::encode(&key), value=hex::encode(&value), "corrupt_rocksdb_item: corrupting item");
                return;
            }
        }
        panic!(
            "cannot corrupt db: cannot find {}-th item in rocksdb column family {}",
            n, cf
        );
    }

    #[test]
    fn test_verify_grovedb_corrupt_0th_root() {
        drive_abci::logging::init_for_tests(LogLevel::Silent);
        let tempdir = tempfile::tempdir().unwrap();
        let db_path = setup_db(tempdir.path());

        corrupt_rocksdb_item(&db_path, "roots", 0);

        let result_error = super::verify_grovedb(&db_path, true).expect_err("expected an error");
        assert_eq!(
            result_error,
            "data corruption error: expected merk to contain value at key 0x08 for tree"
        );

        println!("db path: {:?}", &db_path);
    }
}
