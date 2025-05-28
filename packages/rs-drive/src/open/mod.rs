use crate::cache::SystemDataContracts;
use crate::cache::{DataContractCache, DriveCache, ProtocolVersionsCache};
use crate::config::DriveConfig;
use crate::drive::Drive;
use crate::error::Error;
use dpp::errors::ProtocolError;
use grovedb::GroveDb;
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
        let config = config.unwrap_or_default();

        let grove = Arc::new(GroveDb::open(path)?);

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
                system_data_contracts: SystemDataContracts::load_genesis_system_contracts()?,
            },
        };

        Ok((drive, maybe_platform_version))
    }
}
