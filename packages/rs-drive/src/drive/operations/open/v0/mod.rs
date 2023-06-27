use std::path::Path;
use std::sync::RwLock;
use grovedb::GroveDb;
use crate::drive::cache::{DataContractCache, DriveCache};
use crate::drive::config::DriveConfig;
use crate::drive::Drive;
use crate::drive::system_contracts_cache::SystemContracts;
use crate::error::Error;

impl Drive {
    /// Opens a path in groveDB.
    pub(super) fn open_v0<P: AsRef<Path>>(path: P, config: Option<DriveConfig>) -> Result<Self, Error> {
        match GroveDb::open(path) {
            Ok(grove) => {
                let config = config.unwrap_or_default();
                let genesis_time_ms = config.default_genesis_time;
                let data_contracts_global_cache_size = config.data_contracts_global_cache_size;
                let data_contracts_block_cache_size = config.data_contracts_block_cache_size;

                Ok(Drive {
                    grove,
                    config,
                    system_contracts: SystemContracts::load_system_contracts()?,
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