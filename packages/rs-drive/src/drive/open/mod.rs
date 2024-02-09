use crate::drive::cache::SystemDataContracts;
use crate::drive::cache::{DataContractCache, DriveCache, ProtocolVersionsCache};
use crate::drive::config::DriveConfig;
use crate::drive::defaults::INITIAL_PROTOCOL_VERSION;
use crate::drive::Drive;
use crate::error::Error;
use dpp::errors::ProtocolError;
use dpp::util::deserializer::ProtocolVersion;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;
use std::path::Path;
use std::sync::RwLock;

impl Drive {
    /// Opens GroveDB primary database
    ///
    /// This is a non-versioned method which opens a specified path as a GroveDB instance and returns a `Drive`
    /// instance with this GroveDB, cache and other configurations.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference that implements the `AsRef<Path>` trait. This represents the path to the GroveDB.
    /// * `config` - An `Option` which contains `DriveConfig`. If not specified, default configuration is used.
    /// * `drive_version` - A `DriveVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Self, Error>` - On success, returns `Ok(Self)`, where `Self` is a `Drive` instance. On error, returns an `Error`.
    ///
    pub fn open<P: AsRef<Path>>(
        path: P,
        config: Option<DriveConfig>,
    ) -> Result<(Self, Option<ProtocolVersion>), Error> {
        let grove = GroveDb::open(path)?;

        Self::new_drive_with_grovedb_and_config(grove, config)
    }

    /// Opens GroveDB secondary database
    ///
    /// This is a non-versioned method which opens a specified path as a GroveDB instance and returns a `Drive`
    /// instance with this GroveDB, cache and other configurations.
    ///
    /// # Arguments
    ///
    /// * `primary_path` - A reference that implements the `AsRef<Path>` trait. This represents the path to the
    ///    primary GroveDB database.
    /// * `secondary_path` - A reference that implements the `AsRef<Path>` trait. This represents the path to the
    ///    secondary GroveDB database.
    /// * `config` - An `Option` which contains `DriveConfig`. If not specified, default configuration is used.
    /// * `drive_version` - A `DriveVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Self, Error>` - On success, returns `Ok(Self)`, where `Self` is a `Drive` instance. On error, returns an `Error`.
    ///
    pub fn open_secondary<P: AsRef<Path>>(
        primary_path: P,
        secondary_path: P,
        config: Option<DriveConfig>,
    ) -> Result<(Self, Option<ProtocolVersion>), Error> {
        let grove = GroveDb::open_secondary(primary_path, secondary_path)?;

        Self::new_drive_with_grovedb_and_config(grove, config)
    }

    fn new_drive_with_grovedb_and_config(
        grove: GroveDb,
        config: Option<DriveConfig>,
    ) -> Result<(Self, Option<ProtocolVersion>), Error> {
        let config = config.unwrap_or_default();
        let genesis_time_ms = config.default_genesis_time;
        let data_contracts_global_cache_size = config.data_contracts_global_cache_size;
        let data_contracts_block_cache_size = config.data_contracts_block_cache_size;

        let protocol_version = Drive::fetch_current_protocol_version_with_grovedb(&grove, None)?;

        let platform_version =
            PlatformVersion::get(protocol_version.unwrap_or(INITIAL_PROTOCOL_VERSION))
                .map_err(ProtocolError::PlatformVersionError)?;

        let drive = Drive {
            grove,
            config,
            cache: RwLock::new(DriveCache {
                cached_contracts: DataContractCache::new(
                    data_contracts_global_cache_size,
                    data_contracts_block_cache_size,
                ),
                genesis_time_ms,
                protocol_versions_counter: ProtocolVersionsCache::new(),
                system_data_contracts: SystemDataContracts::load_genesis_system_contracts(
                    platform_version,
                )?,
            }),
        };

        Ok((drive, protocol_version))
    }
}
