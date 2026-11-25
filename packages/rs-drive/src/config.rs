//! Drive Configuration File
//!

use dpp::dashcore::Network;
use dpp::fee::epoch::DEFAULT_EPOCHS_PER_ERA;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Boolean if GroveDB batching consistency verification is enabled by default
pub const DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED: bool = false;
/// Boolean if GroveDB has_raw in enabled by default
pub const DEFAULT_GROVE_HAS_RAW_ENABLED: bool = true;
/// Boolean if verification of GroveDB should be run on startup
pub const DEFAULT_VERIFY_GROVE_ON_STARTUP: bool = false;
/// The default default query limit
pub const DEFAULT_QUERY_LIMIT: u16 = 100;
/// The default max query limit
pub const DEFAULT_MAX_QUERY_LIMIT: u16 = 100;
/// Default maximum number of contracts in cache
pub const DEFAULT_DATA_CONTRACTS_CACHE_SIZE: u64 = 500;
/// The default length of an epoch in seconds: mainnet's, and the node's `ExecutionConfig`
/// default
pub const DEFAULT_EPOCH_TIME_LENGTH_S: u64 = 788400;
/// The default memory budget of GroveDB on LigninDB, in MiB: the capacity of the caches
/// of the live database and of [`DEFAULT_OPEN_CHECKPOINTS`] open checkpoints, together
pub const DEFAULT_LIGNINDB_MEMORY_BUDGET_MB: u64 = 320;
/// The default cache of one open checkpoint on LigninDB, in MiB
pub const DEFAULT_LIGNINDB_CHECKPOINT_CACHE_MB: u64 = 16;
/// The least cache the live database is opened with on LigninDB, in MiB
pub const MIN_LIGNINDB_LIVE_CACHE_MB: u64 = 64;
/// The default reserve the live database keeps for a full disk on LigninDB, in bytes
pub const DEFAULT_LIGNINDB_RESERVE_BYTES: u64 = 1 << 20;
/// The default number of open checkpoints the caches are sized for: the three a node
/// keeps, and the one that is open beside them while the oldest is replaced
pub const DEFAULT_OPEN_CHECKPOINTS: u32 = 4;

/// The storage engine GroveDB keeps its data in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageEngine {
    /// RocksDB
    RocksDb,
    /// LigninDB
    LigninDb,
}

impl std::fmt::Display for StorageEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            StorageEngine::RocksDb => "rocksdb",
            StorageEngine::LigninDb => "lignindb",
        })
    }
}

impl std::str::FromStr for StorageEngine {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "rocksdb" => Ok(StorageEngine::RocksDb),
            "lignindb" => Ok(StorageEngine::LigninDb),
            _ => Err(format!(
                "`{name}` is no storage engine: it is `rocksdb` or `lignindb`"
            )),
        }
    }
}

/// When a commit of LigninDB syncs its log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LigninDbSync {
    /// No commit syncs the log; a flush and a checkpoint do. After a crash the database
    /// is at one of the engine's commits, which need not be the last one.
    Never,
    /// A commit syncs the log when the last sync is at least this many milliseconds ago.
    IntervalMs(u64),
    /// Every commit syncs the log.
    EveryCommit,
}

impl std::fmt::Display for LigninDbSync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LigninDbSync::Never => f.write_str("never"),
            LigninDbSync::IntervalMs(milliseconds) => write!(f, "{milliseconds}"),
            LigninDbSync::EveryCommit => f.write_str("every-commit"),
        }
    }
}

impl std::str::FromStr for LigninDbSync {
    type Err = String;

    fn from_str(policy: &str) -> Result<Self, Self::Err> {
        match policy {
            "never" => Ok(LigninDbSync::Never),
            "every-commit" => Ok(LigninDbSync::EveryCommit),
            _ => policy.parse().map(LigninDbSync::IntervalMs).map_err(|_| {
                format!(
                    "`{policy}` is no sync policy: it is `never`, `every-commit` or a number \
                     of milliseconds"
                )
            }),
        }
    }
}

/// A setting that is written and read as the text its `Display` and `FromStr` give.
#[cfg(feature = "serde")]
macro_rules! serde_as_text {
    ($setting:ty) => {
        impl Serialize for $setting {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $setting {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                String::deserialize(deserializer)?
                    .parse()
                    .map_err(serde::de::Error::custom)
            }
        }
    };
}

#[cfg(feature = "serde")]
serde_as_text!(StorageEngine);
#[cfg(feature = "serde")]
serde_as_text!(LigninDbSync);

/// The caches GroveDB's databases are opened with on LigninDB, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LigninDbCaches {
    /// The cache of the live database
    pub live: usize,
    /// The cache of each open checkpoint
    pub checkpoint: usize,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// Drive configuration struct
pub struct DriveConfig {
    /// Boolean if batching consistency verification is enabled
    #[cfg_attr(
        feature = "serde",
        serde(default = "default_batching_consistency_verification")
    )]
    pub batching_consistency_verification: bool,

    /// Boolean if has_raw is enabled
    #[cfg_attr(feature = "serde", serde(default = "default_has_raw_enabled"))]
    pub has_raw_enabled: bool,

    /// Boolean if GroveDB verification should happen on startup
    #[cfg_attr(
        feature = "serde",
        serde(default = "default_grove_verify_on_startup_enabled")
    )]
    pub grovedb_verify_on_startup: bool,

    /// The default returned count if no limit is set
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_default_query_limit",
            deserialize_with = "from_str_or_number"
        )
    )]
    pub default_query_limit: u16,

    /// The default returned count if no limit is set
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_epochs_per_era",
            deserialize_with = "from_str_or_number"
        )
    )]
    pub epochs_per_era: u16,

    /// The limit for user defined queries
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_max_query_limit",
            deserialize_with = "from_str_or_number"
        )
    )]
    pub max_query_limit: u16,

    /// Default genesis time
    #[cfg_attr(feature = "serde", serde(default))]
    pub default_genesis_time: Option<u64>,

    /// Maximum number of contracts in global cache
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_data_contracts_cache_size",
            deserialize_with = "from_str_or_number"
        )
    )]
    pub data_contracts_global_cache_size: u64,

    /// Maximum number of contracts in block candidate cache
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_data_contracts_cache_size",
            deserialize_with = "from_str_or_number"
        )
    )]
    pub data_contracts_block_cache_size: u64,

    /// The storage engine of GroveDB. A database directory that the other engine made is
    /// refused when it is opened.
    #[cfg_attr(feature = "serde", serde(default = "default_storage_engine"))]
    pub grovedb_storage_engine: StorageEngine,

    /// On LigninDB, the capacity of the caches of the live database and of
    /// `open_checkpoints` open checkpoints together, in MiB. Each open database has a
    /// cache of its own: [`DriveConfig::lignindb_caches`] says how the budget is divided
    /// and what it does not cover.
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_lignindb_memory_budget_mb",
            deserialize_with = "from_text_or_u64"
        )
    )]
    pub grovedb_lignindb_memory_budget_mb: u64,

    /// On LigninDB, the cache of one open checkpoint, in MiB
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_lignindb_checkpoint_cache_mb",
            deserialize_with = "from_text_or_u64"
        )
    )]
    pub grovedb_lignindb_checkpoint_cache_mb: u64,

    /// On LigninDB, when a commit of the live database syncs its log
    #[cfg_attr(feature = "serde", serde(default = "default_lignindb_sync"))]
    pub grovedb_lignindb_sync: LigninDbSync,

    /// On LigninDB, the bytes the live database keeps in reserve, to give back when the
    /// disk is full. With 0 it keeps none, and a reserve that an earlier setting made stays
    /// on disk, unused. A checkpoint keeps none.
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_lignindb_reserve_bytes",
            deserialize_with = "from_text_or_u64"
        )
    )]
    pub grovedb_lignindb_reserve_bytes: u64,

    /// How many checkpoints the caches of LigninDB are sized for. Neither read nor written
    /// with the rest of the config: the node sets it from the number of checkpoints it
    /// keeps when it opens Drive.
    #[cfg_attr(feature = "serde", serde(skip, default = "default_open_checkpoints"))]
    pub open_checkpoints: u32,

    /// GroveDB visualizer address
    #[cfg(feature = "grovedbg")]
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_grovedb_visualizer_address",
            deserialize_with = "from_str_to_socket_address"
        )
    )]
    pub grovedb_visualizer_address: std::net::SocketAddr,

    /// Enable GroveDB visualizer
    #[cfg(feature = "grovedbg")]
    #[cfg_attr(
        feature = "serde",
        serde(default, deserialize_with = "from_str_to_bool")
    )]
    pub grovedb_visualizer_enabled: bool,

    /// The network type
    #[cfg_attr(
        feature = "serde",
        serde(skip_deserializing, default = "DriveConfig::default_network")
    )]
    pub network: Network,

    /// How long an epoch lasts, in seconds. Neither read nor written with the rest of the
    /// config: the node sets it from its execution config's `epoch_time_length_s` when it
    /// opens Drive, so there is one source. Drive reads it only to count the epochs a
    /// document whose type declares a `ttl` has left to live, which its storage fee is paid
    /// out over; the price itself follows the fee schedule's fixed period.
    #[cfg_attr(
        feature = "serde",
        serde(skip, default = "default_epoch_time_length_s")
    )]
    pub epoch_time_length_s: u64,
}

// TODO: some weird envy behavior requires this to exist
#[cfg(all(feature = "serde", feature = "grovedbg"))]
fn from_str_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

#[cfg(feature = "serde")]
fn from_str_or_number<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;
    s.parse::<T>().map_err(Error::custom)
}

/// A number that is read from a number or from its text: a configuration read from the
/// environment gives every value as text.
#[cfg(feature = "serde")]
fn from_text_or_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TextOrNumber {
        Number(u64),
        Text(String),
    }

    match TextOrNumber::deserialize(deserializer)? {
        TextOrNumber::Number(number) => Ok(number),
        TextOrNumber::Text(text) => text.parse().map_err(serde::de::Error::custom),
    }
}

#[cfg(all(feature = "serde", feature = "grovedbg"))]
fn from_str_to_socket_address<'de, D>(deserializer: D) -> Result<std::net::SocketAddr, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

const fn default_batching_consistency_verification() -> bool {
    DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED
}

const fn default_has_raw_enabled() -> bool {
    DEFAULT_GROVE_HAS_RAW_ENABLED
}

const fn default_grove_verify_on_startup_enabled() -> bool {
    DEFAULT_VERIFY_GROVE_ON_STARTUP
}

const fn default_default_query_limit() -> u16 {
    DEFAULT_QUERY_LIMIT
}

const fn default_epochs_per_era() -> u16 {
    DEFAULT_EPOCHS_PER_ERA
}

const fn default_epoch_time_length_s() -> u64 {
    DEFAULT_EPOCH_TIME_LENGTH_S
}

const fn default_max_query_limit() -> u16 {
    DEFAULT_MAX_QUERY_LIMIT
}

const fn default_data_contracts_cache_size() -> u64 {
    DEFAULT_DATA_CONTRACTS_CACHE_SIZE
}

/// The storage engine of a configuration that names none: RocksDB.
///
/// Drive's own unit tests, and a build of Drive with its `test-storage-engine-env`
/// feature, read `DRIVE_TEST_STORAGE_ENGINE` first, so that one build of a test suite
/// runs on either engine. A value that names no engine, text or not, is a panic, so that
/// a run with a mistyped value does not pass on RocksDB. The feature is no default
/// feature, and no build of this repository that ships asks for it. Beside Drive's unit
/// tests, a build reads the variable only where it is asked for the feature, also by
/// another package of the same build.
fn default_storage_engine() -> StorageEngine {
    #[cfg(any(test, feature = "test-storage-engine-env"))]
    {
        match std::env::var("DRIVE_TEST_STORAGE_ENGINE") {
            Ok(named) => {
                return named
                    .parse()
                    .unwrap_or_else(|error| panic!("DRIVE_TEST_STORAGE_ENGINE: {error}"));
            }
            Err(std::env::VarError::NotUnicode(named)) => {
                panic!("DRIVE_TEST_STORAGE_ENGINE: {named:?} names no storage engine")
            }
            Err(std::env::VarError::NotPresent) => {}
        }
    }
    StorageEngine::RocksDb
}

const fn default_lignindb_memory_budget_mb() -> u64 {
    DEFAULT_LIGNINDB_MEMORY_BUDGET_MB
}

const fn default_lignindb_checkpoint_cache_mb() -> u64 {
    DEFAULT_LIGNINDB_CHECKPOINT_CACHE_MB
}

const fn default_lignindb_sync() -> LigninDbSync {
    LigninDbSync::Never
}

const fn default_lignindb_reserve_bytes() -> u64 {
    DEFAULT_LIGNINDB_RESERVE_BYTES
}

const fn default_open_checkpoints() -> u32 {
    DEFAULT_OPEN_CHECKPOINTS
}

/// The default grovedb visualizer_address
pub fn default_grovedb_visualizer_address() -> std::net::SocketAddr {
    "127.0.0.1:8083".parse().unwrap()
}

impl Default for DriveConfig {
    fn default() -> Self {
        DriveConfig {
            batching_consistency_verification: default_batching_consistency_verification(),
            has_raw_enabled: default_has_raw_enabled(),
            grovedb_verify_on_startup: default_grove_verify_on_startup_enabled(),
            default_query_limit: default_default_query_limit(),
            epochs_per_era: default_epochs_per_era(),
            max_query_limit: default_max_query_limit(),
            default_genesis_time: None,
            data_contracts_global_cache_size: default_data_contracts_cache_size(),
            data_contracts_block_cache_size: default_data_contracts_cache_size(),
            grovedb_storage_engine: default_storage_engine(),
            grovedb_lignindb_memory_budget_mb: default_lignindb_memory_budget_mb(),
            grovedb_lignindb_checkpoint_cache_mb: default_lignindb_checkpoint_cache_mb(),
            grovedb_lignindb_sync: default_lignindb_sync(),
            grovedb_lignindb_reserve_bytes: default_lignindb_reserve_bytes(),
            open_checkpoints: default_open_checkpoints(),
            #[cfg(feature = "grovedbg")]
            grovedb_visualizer_address: default_grovedb_visualizer_address(),
            #[cfg(feature = "grovedbg")]
            grovedb_visualizer_enabled: false,
            network: Network::Mainnet,
            epoch_time_length_s: default_epoch_time_length_s(),
        }
    }
}

impl DriveConfig {
    /// The default network type for mainnet
    pub fn default_network() -> Network {
        Network::Mainnet
    }

    /// The default testnet configuration
    pub fn default_testnet() -> Self {
        Self {
            network: Network::Testnet,
            ..Default::default()
        }
    }

    /// The caches of the live database and of each open checkpoint on LigninDB.
    ///
    /// Every open database has a cache of its own. Each of `open_checkpoints` checkpoints
    /// is given `grovedb_lignindb_checkpoint_cache_mb`, and the live database what the
    /// budget has left, which must be [`MIN_LIGNINDB_LIVE_CACHE_MB`] at least.
    ///
    /// What the budget does not cover:
    ///
    /// - A checkpoint that is open beyond that number has its cache outside the budget.
    ///   One is where a reader still holds a checkpoint after it was pruned, and where a
    ///   start finds more checkpoint directories than the node keeps, until the next
    ///   checkpoint prunes them.
    /// - The budget is of the capacities. A cache can hold more than its capacity for a
    ///   time: the pages of an open write transaction, and pages that are not written
    ///   back yet.
    pub fn lignindb_caches(&self) -> Result<LigninDbCaches, String> {
        let budget = self.grovedb_lignindb_memory_budget_mb;
        let checkpoint = self.grovedb_lignindb_checkpoint_cache_mb;
        let checkpoints = u64::from(self.open_checkpoints);
        if checkpoint == 0 {
            return Err("GROVEDB_LIGNINDB_CHECKPOINT_CACHE_MB is 0".to_string());
        }
        let live = checkpoint
            .checked_mul(checkpoints)
            .and_then(|of_checkpoints| budget.checked_sub(of_checkpoints))
            .filter(|live| *live >= MIN_LIGNINDB_LIVE_CACHE_MB)
            .ok_or_else(|| {
                format!(
                    "GROVEDB_LIGNINDB_MEMORY_BUDGET_MB={budget} leaves the live database less \
                     than {MIN_LIGNINDB_LIVE_CACHE_MB} MiB of cache once each of {checkpoints} \
                     checkpoints has GROVEDB_LIGNINDB_CHECKPOINT_CACHE_MB={checkpoint}"
                )
            })?;
        let bytes = |megabytes: u64| {
            megabytes
                .checked_mul(1 << 20)
                .and_then(|bytes| usize::try_from(bytes).ok())
                .ok_or_else(|| format!("a cache of {megabytes} MiB is more than fits in memory"))
        };
        Ok(LigninDbCaches {
            live: bytes(live)?,
            checkpoint: bytes(checkpoint)?,
        })
    }
}

#[cfg(test)]
mod storage_settings_tests {
    use super::*;

    #[test]
    fn the_default_budget_gives_the_live_database_what_the_checkpoints_leave() {
        let config = DriveConfig {
            grovedb_storage_engine: StorageEngine::LigninDb,
            ..Default::default()
        };
        assert_eq!(
            config.lignindb_caches(),
            Ok(LigninDbCaches {
                live: 256 << 20,
                checkpoint: 16 << 20,
            })
        );

        let config = DriveConfig {
            grovedb_lignindb_memory_budget_mb: 1024,
            grovedb_lignindb_checkpoint_cache_mb: 32,
            open_checkpoints: 6,
            ..Default::default()
        };
        let caches = config.lignindb_caches().expect("a budget that is enough");
        assert_eq!(caches.live, (1024 - 6 * 32) << 20);
        assert_eq!(
            caches.live + 6 * caches.checkpoint,
            1024 << 20,
            "the caches together are the budget"
        );
    }

    #[test]
    fn a_budget_that_leaves_the_live_database_too_little_is_refused() {
        let with = |budget, checkpoint, open_checkpoints| DriveConfig {
            grovedb_lignindb_memory_budget_mb: budget,
            grovedb_lignindb_checkpoint_cache_mb: checkpoint,
            open_checkpoints,
            ..Default::default()
        };
        assert!(with(64 + 4 * 16, 16, 4).lignindb_caches().is_ok());
        let refused = with(64 + 4 * 16 - 1, 16, 4)
            .lignindb_caches()
            .expect_err("one MiB short");
        assert!(
            refused.contains("GROVEDB_LIGNINDB_MEMORY_BUDGET_MB=127"),
            "{refused}"
        );
        assert!(
            with(10, 16, 4).lignindb_caches().is_err(),
            "less than the checkpoints take"
        );
        assert!(
            with(u64::MAX, u64::MAX, 4).lignindb_caches().is_err(),
            "an overflow"
        );
        assert!(
            with(320, 0, 4).lignindb_caches().is_err(),
            "a checkpoint with no cache"
        );
        assert!(
            with(u64::MAX, 16, 0).lignindb_caches().is_err(),
            "more than fits in memory"
        );
    }

    #[test]
    fn the_storage_settings_are_read_from_the_text_they_are_written_as() {
        for engine in [StorageEngine::RocksDb, StorageEngine::LigninDb] {
            assert_eq!(engine.to_string().parse(), Ok(engine));
        }
        for sync in [
            LigninDbSync::Never,
            LigninDbSync::IntervalMs(1000),
            LigninDbSync::EveryCommit,
        ] {
            assert_eq!(sync.to_string().parse(), Ok(sync));
        }
        assert!("RocksDB".parse::<StorageEngine>().is_err());
        assert!("always".parse::<LigninDbSync>().is_err());
        assert!("-5".parse::<LigninDbSync>().is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn the_numbers_of_lignindb_are_read_from_numbers_and_from_their_text() {
        #[derive(Deserialize)]
        struct Setting {
            #[serde(deserialize_with = "from_text_or_u64")]
            value: u64,
        }
        for written in [r#"{"value": 320}"#, r#"{"value": "320"}"#] {
            let read: Setting = serde_json::from_str(written).expect("a number");
            assert_eq!(read.value, 320, "{written}");
        }
        for wrong in [
            r#"{"value": "much"}"#,
            r#"{"value": -1}"#,
            r#"{"value": 1.5}"#,
        ] {
            assert!(serde_json::from_str::<Setting>(wrong).is_err(), "{wrong}");
        }

        let config = DriveConfig {
            grovedb_storage_engine: StorageEngine::LigninDb,
            grovedb_lignindb_memory_budget_mb: 512,
            grovedb_lignindb_sync: LigninDbSync::IntervalMs(250),
            ..Default::default()
        };
        let written = serde_json::to_value(&config).expect("the configuration written");
        assert_eq!(written["grovedb_storage_engine"], "lignindb");
        assert_eq!(written["grovedb_lignindb_memory_budget_mb"], 512);
        assert_eq!(written["grovedb_lignindb_sync"], "250");
        assert!(written.get("open_checkpoints").is_none());
    }
}
