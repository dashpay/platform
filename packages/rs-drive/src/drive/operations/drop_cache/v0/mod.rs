use crate::drive::cache::{DataContractCache, ProtocolVersionsCache};
use crate::drive::Drive;

impl Drive {
    /// Drops the drive cache
    pub(super) fn drop_cache_v0(&self) {
        let genesis_time_ms = self.config.default_genesis_time;
        let data_contracts_global_cache_size = self.config.data_contracts_global_cache_size;
        let data_contracts_block_cache_size = self.config.data_contracts_block_cache_size;
        let mut cache = self.cache.write().unwrap();
        cache.cached_contracts = DataContractCache::new(
            data_contracts_global_cache_size,
            data_contracts_block_cache_size,
        );
        cache.genesis_time_ms = genesis_time_ms;
        cache.protocol_versions_counter = ProtocolVersionsCache::new();
    }
}
