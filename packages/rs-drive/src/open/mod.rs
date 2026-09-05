mod load_current_checkpoints;

use crate::cache::SystemDataContracts;
use crate::cache::{DataContractCache, DriveCache, ProtocolVersionsCache};
use crate::config::DriveConfig;
use crate::drive::Drive;
use crate::error::Error;
use dpp::errors::ProtocolError;
use grovedb::GroveDb;
use load_current_checkpoints::load_current_checkpoints;
use platform_version::version::PlatformVersion;
use std::path::Path;
use std::sync::Arc;

impl Drive {
    /// Opens GroveDB database
    ///
    /// This is a non-versioned method which opens a specified path as a GroveDB instance and returns a `Drive`
    /// instance with this GroveDB, cache and other configurations.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference that implements the `AsRef<Path>` trait. This represents the path to the GroveDB.
    /// * `config` - An `Option` which contains `DriveConfig`. If not specified, default configuration is used.
    ///
    /// # Returns
    ///
    /// * `Result<Self, Error>` - On success, returns `Ok(Self)`, where `Self` is a `Drive` instance. On error, returns an `Error`.
    ///
    pub fn open<P: AsRef<Path>>(
        path: P,
        config: Option<DriveConfig>,
    ) -> Result<(Self, Option<&'static PlatformVersion>), Error> {
        let checkpoints_path = path.as_ref().join("checkpoints");
        Self::open_with_checkpoints_path(path, config, checkpoints_path)
    }

    /// Opens GroveDB database, loading the checkpoint registry from an explicit directory.
    ///
    /// Checkpoints may be configured to live outside the database directory
    /// (`CHECKPOINTS_PATH`). Whoever knows that configuration must pass the same directory
    /// checkpoint creation writes to, otherwise the registry comes up empty after a
    /// restart and the checkpoints on disk are neither advertised nor prunable.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the GroveDB.
    /// * `config` - An `Option` which contains `DriveConfig`. If not specified, default configuration is used.
    /// * `checkpoints_path` - The directory checkpoints are written to.
    pub fn open_with_checkpoints_path<P: AsRef<Path>, Q: AsRef<Path>>(
        path: P,
        config: Option<DriveConfig>,
        checkpoints_path: Q,
    ) -> Result<(Self, Option<&'static PlatformVersion>), Error> {
        let config = config.unwrap_or_default();
        let db_path = path.as_ref();

        let grove = Arc::new(GroveDb::open(db_path)?);

        #[cfg(feature = "grovedbg")]
        if config.grovedb_visualizer_enabled {
            grove.start_visualizer(config.grovedb_visualizer_address);
        }
        let genesis_time_ms = config.default_genesis_time;
        let data_contracts_global_cache_size = config.data_contracts_global_cache_size;
        let data_contracts_block_cache_size = config.data_contracts_block_cache_size;

        let maybe_protocol_version =
            Drive::fetch_current_protocol_version_with_grovedb(&grove, None)?;
        let maybe_platform_version = maybe_protocol_version
            .map(|protocol_version| {
                PlatformVersion::get(protocol_version).map_err(ProtocolError::PlatformVersionError)
            })
            .transpose()?;

        // Load existing checkpoints from the configured checkpoints directory
        let checkpoints = load_current_checkpoints(checkpoints_path)?;

        let drive = Drive {
            grove,
            config,
            cache: DriveCache {
                data_contracts: DataContractCache::new(
                    data_contracts_global_cache_size,
                    data_contracts_block_cache_size,
                ),
                genesis_time_ms: parking_lot::RwLock::new(genesis_time_ms),
                protocol_versions_counter: parking_lot::RwLock::new(ProtocolVersionsCache::new()),
                system_data_contracts: SystemDataContracts::new(),
            },
            checkpoints,
        };

        Ok((drive, maybe_platform_version))
    }
}
