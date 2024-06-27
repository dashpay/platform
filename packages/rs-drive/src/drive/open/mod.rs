use std::collections::BTreeMap;
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
use platform_version::error::PlatformVersionError;
use crate::error::drive::DriveError;

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

        let config = config.unwrap_or_default();
        let genesis_time_ms = config.default_genesis_time;
        let data_contracts_global_cache_size = config.data_contracts_global_cache_size;
        let data_contracts_block_cache_size = config.data_contracts_block_cache_size;

        let protocol_version = Drive::fetch_current_protocol_version_with_grovedb(&grove, None)?;

        // At this point we don't know the version what we need to process next block or initialize the chain
        // so version related data should be updated on init chain or on block execution
        let platform_version =
            PlatformVersion::get(protocol_version.unwrap_or(INITIAL_PROTOCOL_VERSION))
                .map_err(ProtocolError::PlatformVersionError)?;

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
                system_data_contracts: SystemDataContracts::load_genesis_system_contracts(
                    platform_version,
                )?,
                // TODO: Populate this from groveDB
                cached_fee_version: parking_lot::RwLock::new(BTreeMap::new()),
            },
        };

        {
            let res_map = drive.get_epochs_protocol_versions(0, None, true, None, platform_version)?;
            let mut write_guard = drive.cache.cached_fee_version.write();

            for (epoch_index, protocol_version) in res_map.iter() {
                let platform_version = PlatformVersion::get(*protocol_version)
                    .map_err(|e| Error::Drive(DriveError::CorruptedCacheState(format!(
                        "no platform version {e}"
                    ))))?;
                write_guard.insert(*epoch_index, &platform_version.fee_version);
            }
        }

        {
            let read_guard = drive.cache.cached_fee_version.read();
        }

        Ok((drive, protocol_version))
    }
}
