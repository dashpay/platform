use crate::cache::ProtocolVersionsCache;
use crate::drive::Drive;

impl Drive {
    /// Drops the drive cache
    pub(crate) fn drop_cache_v0(&self) {
        let genesis_time_ms = self.config.default_genesis_time;
        self.cache.data_contracts.clear();

        let mut genesis_time_ms_cache = self.cache.genesis_time_ms.write();

        *genesis_time_ms_cache = genesis_time_ms;

        drop(genesis_time_ms_cache);

        let mut protocol_versions_counter_cache = self.cache.protocol_versions_counter.write();

        *protocol_versions_counter_cache = ProtocolVersionsCache::new();
    }
}
