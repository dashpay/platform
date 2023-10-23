use crate::drive::cache::{DataContractCache, DriveCache};
use crate::drive::config::DriveConfig;
use crate::drive::system_contracts_cache::SystemContracts;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::GroveDb;
use std::path::Path;
use std::sync::RwLock;

impl Drive {
    /// Opens a path in GroveDB.
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
    pub fn open<P: AsRef<Path>>(path: P, config: Option<DriveConfig>) -> Result<Self, Error> {
        match GroveDb::open(path) {
            Ok(grove) => {
                let config = config.unwrap_or_default();
                let genesis_time_ms = config.default_genesis_time;
                let data_contracts_global_cache_size = config.data_contracts_global_cache_size;
                let data_contracts_block_cache_size = config.data_contracts_block_cache_size;

                Ok(Drive {
                    grove,
                    config,
                    //todo: BEFORE MAINNET move this outside of open
                    system_contracts: SystemContracts::load_genesis_system_contracts(1)?,
                    cache: RwLock::new(DriveCache {
                        cached_contracts: DataContractCache::new(
                            data_contracts_global_cache_size,
                            data_contracts_block_cache_size,
                        ),
                        genesis_time_ms,
                        protocol_versions_counter: None,
                    }),
                })
            }
            Err(e) => Err(Error::GroveDB(e)),
        }
    }
}
