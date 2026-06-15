//! Main server process for RS-Drive-ABCI
//!
//! RS-Drive-ABCI server starts a single-threaded server and listens to connections from Tenderdash.
#[cfg(feature = "replay")]
use drive_abci::replay::{self, ReplayArgs};
use drive_abci::verify::verify_grovedb;

use clap::{Parser, Subcommand};
use dapi_grpc::platform::v0::get_status_request;
use dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0;
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::tonic::transport::Uri;
use drive_abci::config::{FromEnv, PlatformConfig};
use drive_abci::core::wait_for_core_to_sync::v0::wait_for_core_to_sync_v0;
use drive_abci::logging::{LogBuilder, LogConfig, LogDestination, Loggers};
use drive_abci::metrics::Prometheus;
use drive_abci::platform_types::platform::Platform;
use drive_abci::rpc::core::DefaultCoreRPC;
use drive_abci::{logging, server};
use itertools::Itertools;
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

    /// Replay ABCI requests captured from drive-abci logs.
    #[cfg(feature = "replay")]
    #[command()]
    Replay(ReplayArgs),

    /// Produce a shielded-pool snapshot file at `--out` by running the full
    /// genesis + seed cycle against a fresh temporary GroveDB, then dumping
    /// the resulting subtree. Self-contained — does not need a running
    /// drive-abci or a populated DB.
    ///
    /// Intended for the Dockerfile bake stage, where the snapshot file is
    /// embedded into the runtime image and consumed at boot via
    /// `DRIVE_SHIELDED_SNAPSHOT=<path>`. Requires the binary to be built
    /// with `--features=shielded_test_data` — the command is compiled out
    /// otherwise.
    #[cfg(feature = "shielded_test_data")]
    #[command()]
    SnapshotBake {
        /// Where to write the snapshot file. Parent directory must exist.
        #[arg(long)]
        out: PathBuf,
    },
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

                // Pre-build the shielded verifying key on a background thread so
                // the first shielded transaction doesn't pay the ~5-15s build cost.
                std::thread::spawn(|| {
                    use drive_abci::execution::validation::state_transition::shielded_common::warmup_shielded_verifying_key;
                    tracing::info!("pre-building shielded verifying key in background");
                    warmup_shielded_verifying_key();
                    tracing::info!("shielded verifying key is ready");
                });

                server::start(runtime, Arc::new(platform), config, cancel);

                tracing::info!("drive-abci server is stopped");

                return Ok(());
            }
            Commands::Config => dump_config(&config)?,
            Commands::Status => runtime.block_on(check_status(&config))?,
            Commands::Verify => drive_abci::verify::run(&config, true)?,
            #[cfg(feature = "shielded_test_data")]
            Commands::SnapshotBake { out } => snapshot_bake_main::run(&config, &out)?,
            Commands::Version => print_version(),
            #[cfg(feature = "replay")]
            Commands::Replay(args) => {
                replay::run(config, args, cancel.clone()).map_err(|e| e.to_string())?;
                return Ok(());
            }
        };

        Ok(())
    }
}

fn main() -> Result<(), ExitCode> {
    let cli = Cli::parse();
    // SnapshotBake runs against an in-container tempdir with no chain env —
    // skip `load_config` (which would panic on missing GRPC_BIND_ADDRESS etc.)
    // and use a sensible default. Other subcommands (Start / Status / etc.)
    // still need the full config. The command only exists under
    // `feature = "shielded_test_data"`, so the branch is compiled out otherwise.
    #[cfg(feature = "shielded_test_data")]
    let config = if matches!(cli.command, Commands::SnapshotBake { .. }) {
        drive_abci::config::PlatformConfig::default_local()
    } else {
        load_config(&cli.config)
    };
    #[cfg(not(feature = "shielded_test_data"))]
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

/// Everything that exists only to support the `snapshot-bake` subcommand
/// (used by the Dockerfile bake stage to pre-build a shielded-pool snapshot
/// for the runtime image to apply at InitChain). Gated as a whole on the
/// `shielded_test_data` Cargo feature so production builds carry none of it.
#[cfg(feature = "shielded_test_data")]
mod snapshot_bake_main {
    use dpp::dashcore::ephemerealdata::chain_lock::ChainLock;
    use dpp::dashcore::{Block, BlockHash, Header, InstantLock, QuorumHash, Transaction, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        AssetUnlockStatusResult, ExtendedQuorumListResult, GetChainTipsResult, MasternodeListDiff,
        MnSyncStatus, QuorumInfoResult, QuorumType, SoftforkInfo,
    };
    use dpp::dashcore_rpc::json::GetRawTransactionResult;
    use dpp::dashcore_rpc::Error;
    use dpp::prelude::TimestampMillis;
    use dpp::version::PlatformVersion;
    use drive_abci::config::PlatformConfig;
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::rpc::core::CoreRPCLike;
    use serde_json::Value;

    /// Stub CoreRPCLike — Platform::open_with_client requires a CoreRPCLike,
    /// but create_genesis_state never actually touches Core (no chain locks,
    /// transactions, or quorum lookups happen during genesis). Every method
    /// is `unreachable!()` so a bake that accidentally tries to talk to Core
    /// surfaces as a loud panic.
    pub(super) struct NoopCoreRPC;

    impl CoreRPCLike for NoopCoreRPC {
        fn get_block_hash(&self, _: u32) -> Result<BlockHash, Error> {
            unreachable!()
        }
        fn get_block_header(&self, _: &BlockHash) -> Result<Header, Error> {
            unreachable!()
        }
        fn get_block_time_from_height(&self, _: u32) -> Result<TimestampMillis, Error> {
            unreachable!()
        }
        fn get_best_chain_lock(&self) -> Result<ChainLock, Error> {
            unreachable!()
        }
        fn submit_chain_lock(&self, _: &ChainLock) -> Result<u32, Error> {
            unreachable!()
        }
        fn get_transaction(&self, _: &Txid) -> Result<Transaction, Error> {
            unreachable!()
        }
        fn get_asset_unlock_statuses(
            &self,
            _: &[u64],
            _: u32,
        ) -> Result<Vec<AssetUnlockStatusResult>, Error> {
            unreachable!()
        }
        fn get_transaction_extended_info(
            &self,
            _: &Txid,
        ) -> Result<GetRawTransactionResult, Error> {
            unreachable!()
        }
        fn get_fork_info(&self, _: &str) -> Result<Option<SoftforkInfo>, Error> {
            unreachable!()
        }
        fn get_block(&self, _: &BlockHash) -> Result<Block, Error> {
            unreachable!()
        }
        fn get_block_json(&self, _: &BlockHash) -> Result<Value, Error> {
            unreachable!()
        }
        fn get_chain_tips(&self) -> Result<GetChainTipsResult, Error> {
            unreachable!()
        }
        fn get_quorum_listextended(
            &self,
            _: Option<u32>,
        ) -> Result<ExtendedQuorumListResult, Error> {
            unreachable!()
        }
        fn get_quorum_info(
            &self,
            _: QuorumType,
            _: &QuorumHash,
            _: Option<bool>,
        ) -> Result<QuorumInfoResult, Error> {
            unreachable!()
        }
        fn get_protx_diff_with_masternodes(
            &self,
            _: Option<u32>,
            _: u32,
        ) -> Result<MasternodeListDiff, Error> {
            unreachable!()
        }
        fn verify_instant_lock(&self, _: &InstantLock, _: Option<u32>) -> Result<bool, Error> {
            unreachable!()
        }
        fn verify_chain_lock(&self, _: &ChainLock) -> Result<bool, Error> {
            unreachable!()
        }
        fn masternode_sync_status(&self) -> Result<MnSyncStatus, Error> {
            unreachable!()
        }
        fn send_raw_transaction(&self, _: &[u8]) -> Result<Txid, Error> {
            unreachable!()
        }
    }

    /// Produce a shielded-pool snapshot at `out_path` from a fresh temporary
    /// GroveDB. Runs the full `create_genesis_state` cycle (which, under
    /// `feature = "shielded_test_data"`, invokes the shielded-pool seeder),
    /// then dumps the resulting subtree. Self-contained — `_config` is
    /// ignored (we use a tempdir + sensible defaults).
    ///
    /// Intended for the Dockerfile bake stage: produce a snapshot once during
    /// image build, embed in the runtime image, load it at every InitChain
    /// via `DRIVE_SHIELDED_SNAPSHOT`.
    pub(super) fn run(_config: &PlatformConfig, out_path: &std::path::Path) -> Result<(), String> {
        tracing::info!(
            out = %out_path.display(),
            "snapshot-bake: creating tempdir + bootstrapping fresh GroveDB",
        );

        let tempdir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;

        // Use the local (regtest) config — same network the bake target image
        // will run on. We use NoopCoreRPC so we don't try to connect to a
        // non-existent Core node during the in-container bake.
        let mut platform_config = PlatformConfig::default_local();
        platform_config.db_path = tempdir.path().to_path_buf();

        let platform = Platform::<NoopCoreRPC>::open_with_client(
            tempdir.path(),
            Some(platform_config),
            NoopCoreRPC,
            None,
        )
        .map_err(|e| format!("open platform: {e}"))?;

        let platform_version = PlatformVersion::latest();
        let tx = platform.drive.grove.start_transaction();

        // Defensively unset DRIVE_SHIELDED_SNAPSHOT before seeding. The seeder
        // (`create_data_for_shielded_pool`) checks this env var first and, if
        // set, APPLIES the referenced snapshot instead of running the seeder.
        // A developer (or the Dockerfile env) with it exported would make
        // `snapshot-bake` recursively re-dump an inherited snapshot rather
        // than seeding a fresh one.
        std::env::remove_var("DRIVE_SHIELDED_SNAPSHOT");

        tracing::info!("snapshot-bake: running create_genesis_state (seeds shielded pool under feature = \"shielded_test_data\")");
        platform
            .create_genesis_state(
                1, // genesis_core_height (placeholder for bake)
                0, // genesis_time (placeholder for bake)
                Some(&tx),
                platform_version,
            )
            .map_err(|e| format!("create_genesis_state: {e}"))?;
        tx.commit().map_err(|e| format!("commit: {e}"))?;

        tracing::info!(
            out = %out_path.display(),
            "snapshot-bake: dumping shielded subtree to snapshot file",
        );
        let stats = drive_abci::shielded_snapshot::dump_shielded_subtree(
            &platform.drive.grove,
            None,
            out_path,
            platform_version,
        )
        .map_err(|e| format!("snapshot dump failed: {e}"))?;

        tracing::info!(
            out = %out_path.display(),
            total_count = stats.total_count,
            key_count = stats.key_count,
            sst_bytes = stats.sst_bytes,
            "snapshot-bake: wrote shielded-pool snapshot",
        );
        println!(
            "wrote {} bytes ({} keys, total_count={}) to {}",
            stats.sst_bytes,
            stats.key_count,
            stats.total_count,
            out_path.display(),
        );

        Ok(())
    }
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
    }
}
