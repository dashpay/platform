use crate::drive::contract::DataContractFetchInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use moka::sync::Cache;
use std::sync::Arc;

/// DataContract cache that handles both global and block data
pub struct DataContractCache {
    global_cache: Cache<[u8; 32], Arc<DataContractFetchInfo>>,
    block_cache: Cache<[u8; 32], Arc<DataContractFetchInfo>>,
}

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
    pub fn insert(&self, fetch_info: Arc<DataContractFetchInfo>, is_block_cache: bool) {
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
    pub fn remove(&self, contract_id: [u8; 32]) {
        self.block_cache.remove(&contract_id);
        self.global_cache.remove(&contract_id);
    }

    /// Move block cache to global cache
    pub fn merge_and_clear_block_cache(&self) {
        for (contract_id, fetch_info) in self.block_cache.into_iter() {
            self.global_cache
                .insert(Arc::unwrap_or_clone(contract_id), fetch_info);
        }
        self.clear_block_cache();
    }

    /// Clear block cache
    pub fn clear_block_cache(&self) {
        self.block_cache.invalidate_all();
    }

    /// Clear cache
    pub fn clear(&self) {
        self.block_cache.invalidate_all();
        self.global_cache.invalidate_all();
    }
}

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
