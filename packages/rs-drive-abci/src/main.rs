//! Main server process for RS-Drive-ABCI
//!
//! RS-Drive-ABCI server starts a single-threaded server and listens to connections from Tenderdash.

mod server;

use clap::{Parser, Subcommand};
use drive_abci::abci;
use drive_abci::config::{FromEnv, PlatformConfig};
use drive_abci::core::wait_for_core_to_sync::v0::wait_for_core_to_sync_v0;
use drive_abci::logging;
use drive_abci::logging::{LogBuilder, LogConfig, LogDestination, Loggers};
use drive_abci::metrics::{Prometheus, DEFAULT_PROMETHEUS_PORT};
use drive_abci::platform_types::platform::Platform;
use drive_abci::rpc::core::DefaultCoreRPC;
use itertools::Itertools;
use std::fs::remove_file;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::runtime::{Builder, Runtime};
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

    /// Verify integrity of database.
    ///
    /// This command will execute GroveDB hash integrity checks.
    ///
    /// You can also enforce grovedb integrity checks during `drive-abci start`
    /// by creating `.fsck` file in database directory (`DB_PATH`).
    #[command()]
    Verify,
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
                verify_grovedb(&config.db_path, false)?;

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
                //wait_for_core_to_sync_v0(&core_rpc, cancel.clone()).map_err(|e| e.to_string())?;

                if cancel.is_cancelled() {
                    return Ok(());
                }

                let platform: Platform<DefaultCoreRPC> = Platform::open_with_client(
                    config.db_path.clone(),
                    Some(config.clone()),
                    core_rpc,
                )
                .expect("Failed to open platform");

                server::start(runtime, Arc::new(platform), config, cancel);

                return Ok(());
            }
            Commands::Config => dump_config(&config)?,
            Commands::Status => check_status(&config)?,
            Commands::Verify => verify_grovedb(&config.db_path, true)?,
        };

        Ok(())
    }
}

fn main() -> Result<(), ExitCode> {
    let cli = Cli::parse();
    let config = load_config(&cli.config);

    // Start tokio runtime and thread listening for signals.
    // The runtime will be reused by Prometheus and rs-tenderdash-abci.
    // TODO: We might want to limit worker threads
    // TODO: Figure out how many blocking threads and grpc concurrency we should set
    let runtime = Builder::new_multi_thread()
        // TODO: 8 MB stack threads as some recursions in GroveDB can be pretty deep
        //  We could remove such a stack stack size once deletion of a node doesn't recurse in grovedb
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        // TODO: We probably we want to have them bigger than concurrency limit in tonic to make
        //  sure that other libraries have room to spawn them
        // TODO: Expose limits as configuration so we can easily tune them without rebuilding
        // .max_blocking_threads(num_cpus::get() * 5)
        .build()
        .expect("cannot initialize tokio runtime");

    // We use `cancel` to notify other subsystems that the server is shutting down
    let cancel = CancellationToken::new();

    let loggers = configure_logging(&cli, &config).expect("failed to configure logging");

    install_panic_hook(cancel.clone());

    let runtime_guard = runtime.enter();

    runtime.spawn(handle_signals(cancel.clone(), loggers));

    let result = match cli.run(&runtime, config, cancel) {
        Ok(()) => {
            tracing::debug!("shutdown complete");
            Ok(())
        }
        Err(e) => {
            tracing::error!(error = e, "drive-abci failed");
            Err(ExitCode::FAILURE)
        }
    };

    drop(runtime_guard);
    runtime.shutdown_timeout(Duration::from_millis(SHUTDOWN_TIMEOUT_MILIS));
    tracing::info!("drive-abci server is stopped");

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

/// Check status of ABCI server.
fn check_status(config: &PlatformConfig) -> Result<(), String> {
    if let Some(prometheus_addr) = &config.prometheus_bind_address {
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
    let result = grovedb
        .visualize_verify_grovedb()
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

fn configure_logging(cli: &Cli, config: &PlatformConfig) -> Result<Loggers, logging::Error> {
    let mut configs = config.abci.log.clone();
    if configs.is_empty() || cli.verbose > 0 {
        let cli_config = LogConfig {
            destination: LogDestination::StdOut,
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

#[cfg(test)]
mod test {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use ::drive::{drive::Drive, fee_pools::epochs::paths::EpochProposers, query::Element};
    use dpp::block::epoch::Epoch;
    use drive::fee_pools::epochs::epoch_key_constants;

    use dpp::version::PlatformVersion;
    use drive_abci::logging::LogLevel;
    use rocksdb::{IteratorMode, Options};

    /// Setup drive database by creating initial state structure and inserting some data.
    ///
    /// Returns path to the database.
    fn setup_db(tempdir: &Path) -> PathBuf {
        let path = tempdir.join("db");
        fs::create_dir(&path).expect("create db dir");

        let (drive, _) = Drive::open(&path, None).expect("open drive");

        let platform_version = PlatformVersion::latest();

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

        // let iter = db.iterator(IteratorMode::Start);
        for (i, item) in iter.enumerate() {
            let (key, mut value) = item.unwrap();
            // println!("{} = {}", hex::encode(&key), hex::encode(value));
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

        let result = super::verify_grovedb(&db_path, true);
        assert!(result.is_err());

        println!("db path: {:?}", &db_path);
    }
}
