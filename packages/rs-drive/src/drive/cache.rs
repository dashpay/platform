pub use crate::drive::cache::system_contracts_cache::SystemDataContracts;
#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
#[cfg(any(feature = "full", feature = "verify"))]
use dpp::identity::TimestampMillis;
#[cfg(any(feature = "full", feature = "verify"))]
use dpp::util::deserializer::ProtocolVersion;
use grovedb::TransactionArg;
#[cfg(any(feature = "full", feature = "verify"))]
use moka::sync::Cache;
#[cfg(any(feature = "full", feature = "verify"))]
use nohash_hasher::IntMap;
use platform_version::version::drive_versions::DriveVersion;
#[cfg(any(feature = "full", feature = "verify"))]
use std::sync::Arc;

#[cfg(feature = "full")]
mod system_contracts_cache;

/// Drive cache struct
#[cfg(feature = "full")]
pub struct DriveCache {
    /// Cached contracts
    pub cached_contracts: DataContractCache,
    /// Genesis time in ms
    pub genesis_time_ms: Option<TimestampMillis>,
    /// Lazy loaded counter of votes to upgrade protocol version
    pub protocol_versions_counter: ProtocolVersionsCache,
    /// Versioned system data contracts
    pub system_data_contracts: SystemDataContracts,
}

/// ProtocolVersion cache that handles both global and block data
#[cfg(feature = "full")]
#[derive(Default)]
pub struct ProtocolVersionsCache {
    /// The current global cache for protocol versions
    // TODO: If we persist this in the state and it should be loaded for correct
    //  use then it's not actually the cache. Move out of cache because it's confusing
    pub global_cache: IntMap<ProtocolVersion, u64>,
    block_cache: IntMap<ProtocolVersion, u64>,
    loaded: bool,
    needs_wipe: bool,
}

/// DataContract cache that handles both global and block data
#[cfg(feature = "full")]
pub struct DataContractCache {
    global_cache: Cache<[u8; 32], Arc<DataContractFetchInfo>>,
    block_cache: Cache<[u8; 32], Arc<DataContractFetchInfo>>,
}

#[cfg(feature = "full")]
impl ProtocolVersionsCache {
    /// Create a new ProtocolVersionsCache instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Load the protocol versions cache from disk if needed
    pub fn load_if_needed(
        &mut self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        if !self.loaded {
            self.global_cache = drive.fetch_versions_with_counter(transaction, drive_version)?;
            self.loaded = true;
        };
        Ok(())
    }

    /// Sets the protocol version to the block cache
    pub fn set_block_cache_version_count(&mut self, version: ProtocolVersion, count: u64) {
        self.block_cache.insert(version, count);
    }

    /// Tries to get a version from block cache if present
    /// if block cache doesn't have the version set
    /// then it tries get the version from global cache
    pub fn get(&self, version: &ProtocolVersion) -> Option<&u64> {
        if let Some(count) = self.block_cache.get(version) {
            Some(count)
        } else {
            self.global_cache.get(version)
        }
    }

    /// Merge block cache to global cache
    pub fn merge_block_cache(&mut self) {
        if self.needs_wipe {
            self.global_cache.clear();
            self.block_cache.clear();
            self.needs_wipe = false;
        } else {
            self.global_cache.extend(self.block_cache.drain());
        }
    }

    /// Clear block cache
    pub fn clear_block_cache(&mut self) {
        self.block_cache.clear()
    }

    /// Versions passing threshold
    pub fn versions_passing_threshold(
        &mut self,
        required_upgraded_hpns: u64,
    ) -> Vec<ProtocolVersion> {
        self.needs_wipe = true;
        let mut cache = self.global_cache.clone();

        cache.extend(self.block_cache.drain());
        cache
            .into_iter()
            .filter_map(|(protocol_version, count)| {
                if count >= required_upgraded_hpns {
                    Some(protocol_version)
                } else {
                    None
                }
            })
            .collect::<Vec<ProtocolVersion>>()
    }
}

#[cfg(feature = "full")]
impl DataContractCache {
    /// Create a new DataContract cache instance
    pub fn new(global_cache_max_capacity: u64, block_cache_max_capacity: u64) -> Self {
        Self {
            global_cache: Cache::new(global_cache_max_capacity),
            block_cache: Cache::new(block_cache_max_capacity),
        }
    }

    /// Inserts DataContract to block cache
    /// otherwise to goes to global cache
    pub fn insert(&mut self, fetch_info: Arc<DataContractFetchInfo>, is_block_cache: bool) {
        let data_contract_id_bytes = fetch_info.contract.id().to_buffer();

        if is_block_cache {
            self.block_cache.insert(data_contract_id_bytes, fetch_info);
        } else {
            self.global_cache.insert(data_contract_id_bytes, fetch_info);
        }
    }

    /// Tries to get a data contract from block cache if present
    /// if block cache doesn't have the contract
    /// then it tries get the contract from global cache
    pub fn get(
        &self,
        contract_id: [u8; 32],
        is_block_cache: bool,
    ) -> Option<Arc<DataContractFetchInfo>> {
        let maybe_fetch_info = if is_block_cache {
            self.block_cache.get(&contract_id)
        } else {
            None
        };

        maybe_fetch_info.or_else(|| self.global_cache.get(&contract_id))
    }

    /// Remove contract from both block and global cache
    pub fn remove(&mut self, contract_id: [u8; 32]) {
        self.block_cache.remove(&contract_id);
        self.global_cache.remove(&contract_id);
    }

    /// Merge block cache to global cache
    pub fn merge_block_cache(&mut self) {
        for (contract_id, fetch_info) in self.block_cache.iter() {
            self.global_cache.insert(*contract_id, fetch_info);
        }
    }

    /// Clear block cache
    pub fn clear_block_cache(&mut self) {
        self.block_cache.invalidate_all();
    }
}

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use super::*;

    mod get {
        use super::*;
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
        use dpp::version::PlatformVersion;

        #[test]
        fn test_get_from_global_cache_when_block_cache_is_not_requested() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let protocol_version = PlatformVersion::latest().protocol_version;

            // Create global contract
            let fetch_info_global = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                protocol_version,
            ));

            let contract_id = fetch_info_global.contract.id().to_buffer();

            data_contract_cache
                .global_cache
                .insert(contract_id, Arc::clone(&fetch_info_global));

            // Create transactional contract with a new version
            let mut fetch_info_block =
                DataContractFetchInfo::dpns_contract_fixture(protocol_version);

            fetch_info_block.contract.increment_version();

            let fetch_info_block_boxed = Arc::new(fetch_info_block);

            data_contract_cache
                .block_cache
                .insert(contract_id, Arc::clone(&fetch_info_block_boxed));

            let fetch_info_from_cache = data_contract_cache
                .get(contract_id, false)
                .expect("should be present");

            assert_eq!(fetch_info_from_cache, fetch_info_global)
        }

        #[test]
        fn test_get_from_global_cache_when_block_cache_does_not_have_contract() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let protocol_version = PlatformVersion::latest().protocol_version;

            let fetch_info_global = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                protocol_version,
            ));

            let contract_id = fetch_info_global.contract.id().to_buffer();

            data_contract_cache
                .global_cache
                .insert(contract_id, Arc::clone(&fetch_info_global));

            let fetch_info_from_cache = data_contract_cache
                .get(contract_id, true)
                .expect("should be present");

            assert_eq!(fetch_info_from_cache, fetch_info_global)
        }

        #[test]
        fn test_get_from_block_cache() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let protocol_version = PlatformVersion::latest().protocol_version;

            let fetch_info_block = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                protocol_version,
            ));

            let contract_id = fetch_info_block.contract.id().to_buffer();

            data_contract_cache
                .block_cache
                .insert(contract_id, Arc::clone(&fetch_info_block));

            let fetch_info_from_cache = data_contract_cache
                .get(contract_id, true)
                .expect("should be present");

            assert_eq!(fetch_info_from_cache, fetch_info_block)
        }
    }
}
