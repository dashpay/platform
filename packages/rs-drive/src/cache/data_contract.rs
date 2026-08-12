use crate::drive::contract::DataContractFetchInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use moka::ops::compute::Op;
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
    ///
    /// The insert is skipped if the cache already holds the same contract at a
    /// **higher** version. Contract versions increase strictly monotonically —
    /// the data contract update transition enforces `new == old + 1`, and system
    /// contract migrations bump the version — so an insert carrying a lower
    /// version is always a delayed writer racing a newer copy in, never fresh
    /// information. CONSENSUS-CRITICAL: a read-only query thread fetches from
    /// committed state without a transaction and populates the global cache from
    /// what it read. If such a thread reads a contract, gets descheduled while
    /// block execution rewrites that contract, and performs its insert after the
    /// block cache is promoted, an unconditional insert would clobber the newer
    /// contract with the stale one — and block execution would then serialize
    /// documents against a different contract than a node whose cache was cold,
    /// producing a different app hash from the same block. The check-and-insert
    /// is atomic per key via moka's compute API, so there is no window between
    /// the version comparison and the write. Same-version inserts still
    /// overwrite: re-inserting an identical contract with a freshly calculated
    /// fee is the normal cache-hit fee path.
    pub fn insert(&self, fetch_info: Arc<DataContractFetchInfo>, is_block_cache: bool) {
        let data_contract_id_bytes = fetch_info.contract.id().to_buffer();

        let cache = if is_block_cache {
            &self.block_cache
        } else {
            &self.global_cache
        };

        cache
            .entry(data_contract_id_bytes)
            .and_compute_with(|existing| match existing {
                Some(entry) if entry.value().contract.version() > fetch_info.contract.version() => {
                    Op::Nop
                }
                _ => Op::Put(Arc::clone(&fetch_info)),
            });
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
    use dpp::version::PlatformVersion;

    mod get {
        use super::*;
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};

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

    mod insert {
        use super::*;
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};

        /// Two copies of the SAME contract (same id) at the given versions. The
        /// fixture generates a fresh contract id per call, so both copies must
        /// derive from a single fixture.
        fn same_contract_at_versions(
            first: u32,
            second: u32,
        ) -> (Arc<DataContractFetchInfo>, Arc<DataContractFetchInfo>) {
            let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
                PlatformVersion::latest().protocol_version,
            );
            let mut first_info = fetch_info.clone();
            first_info.contract.set_version(first);
            let mut second_info = fetch_info;
            second_info.contract.set_version(second);
            (Arc::new(first_info), Arc::new(second_info))
        }

        /// A delayed insert carrying an older contract version must not clobber a newer
        /// entry. This is the query-thread race: a read-only query reads a contract from
        /// committed state, is descheduled while block execution rewrites the contract,
        /// and performs its cache insert only after the migrated contract was promoted
        /// to the global cache.
        #[test]
        fn test_insert_does_not_overwrite_newer_version_with_older() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (stale, newer) = same_contract_at_versions(1, 2);
            let contract_id = newer.contract.id().to_buffer();
            data_contract_cache.insert(newer, false);

            // The delayed stale insert
            data_contract_cache.insert(stale, false);

            let cached = data_contract_cache
                .get(contract_id, false)
                .expect("should be present");
            assert_eq!(cached.contract.version(), 2);
        }

        /// Same-version inserts must overwrite: re-inserting the same contract with a
        /// freshly calculated fee is the normal cache-hit fee path.
        #[test]
        fn test_insert_overwrites_same_version() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (mut original, mut with_fee) = same_contract_at_versions(1, 1);
            let contract_id = original.contract.id().to_buffer();
            Arc::make_mut(&mut original).fee = None;
            data_contract_cache.insert(original, false);

            Arc::make_mut(&mut with_fee).fee =
                Some(dpp::fee::fee_result::FeeResult::new_from_processing_fee(1));
            data_contract_cache.insert(with_fee, false);

            let cached = data_contract_cache
                .get(contract_id, false)
                .expect("should be present");
            assert!(cached.fee.is_some(), "same-version insert must overwrite");
        }

        /// The full race, end to end: block execution seeds the migrated (newer)
        /// contract into the block cache, the block finalizes and promotes it to the
        /// global cache, and only then does the delayed query thread insert the
        /// pre-migration contract it read before the rewrite. The promoted contract
        /// must survive.
        #[test]
        fn test_delayed_stale_insert_after_promotion_does_not_stick() {
            let data_contract_cache = DataContractCache::new(10, 10);

            // The pre-migration contract, as read from committed state by a query
            // thread that will be descheduled before its insert.
            let (stale, migrated) = same_contract_at_versions(1, 2);
            let contract_id = stale.contract.id().to_buffer();

            // Block execution writes the migrated contract and seeds the block cache.
            data_contract_cache.insert(migrated, true);

            // The block finalizes: block cache promotes to global.
            data_contract_cache.merge_and_clear_block_cache();

            // The query thread wakes up and performs its stale insert.
            data_contract_cache.insert(stale, false);

            let cached = data_contract_cache
                .get(contract_id, false)
                .expect("should be present");
            assert_eq!(
                cached.contract.version(),
                2,
                "the promoted migrated contract must survive a delayed stale insert"
            );
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn test_remove_clears_global_cache_entry() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let protocol_version = PlatformVersion::latest().protocol_version;
            let fetch_info = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                protocol_version,
            ));
            let contract_id = fetch_info.contract.id().to_buffer();

            data_contract_cache.insert(fetch_info, false);
            data_contract_cache.remove(contract_id);

            assert!(data_contract_cache.get(contract_id, false).is_none());
        }

        #[test]
        fn test_remove_clears_entry_from_both_caches() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let protocol_version = PlatformVersion::latest().protocol_version;
            let fetch_info_global = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                protocol_version,
            ));
            let contract_id = fetch_info_global.contract.id().to_buffer();
            let fetch_info_block = Arc::clone(&fetch_info_global);

            data_contract_cache.insert(fetch_info_global, false);
            data_contract_cache.insert(fetch_info_block, true);
            data_contract_cache.remove(contract_id);

            assert!(data_contract_cache.block_cache.get(&contract_id).is_none());
            assert!(data_contract_cache.global_cache.get(&contract_id).is_none());
        }
    }

    mod merge_and_clear_block_cache {
        use super::*;

        #[test]
        fn test_merge_moves_block_items_to_global_cache() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let protocol_version = PlatformVersion::latest().protocol_version;
            let fetch_info = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                protocol_version,
            ));
            let contract_id = fetch_info.contract.id().to_buffer();

            data_contract_cache.insert(fetch_info, true);
            data_contract_cache.merge_and_clear_block_cache();

            assert!(data_contract_cache.global_cache.get(&contract_id).is_some());
        }

        #[test]
        fn test_merge_clears_block_cache() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let protocol_version = PlatformVersion::latest().protocol_version;
            let fetch_info = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                protocol_version,
            ));
            let contract_id = fetch_info.contract.id().to_buffer();

            data_contract_cache.insert(fetch_info, true);
            data_contract_cache.merge_and_clear_block_cache();

            assert!(data_contract_cache.block_cache.get(&contract_id).is_none());
        }
    }

    mod clear {
        use super::*;

        #[test]
        fn test_clear_empties_global_and_block_caches() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let protocol_version = PlatformVersion::latest().protocol_version;
            let fetch_info_global = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                protocol_version,
            ));
            let contract_id = fetch_info_global.contract.id().to_buffer();
            let fetch_info_block = Arc::clone(&fetch_info_global);

            data_contract_cache.insert(fetch_info_global, false);
            data_contract_cache.insert(fetch_info_block, true);
            data_contract_cache.clear();

            assert!(data_contract_cache.get(contract_id, false).is_none());
            assert!(data_contract_cache.block_cache.get(&contract_id).is_none());
        }
    }
}
