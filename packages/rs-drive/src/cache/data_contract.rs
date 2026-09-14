use crate::drive::contract::DataContractFetchInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use moka::ops::compute::Op;
use moka::sync::Cache;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// How many blocks the cache had seen committed when the snapshot was taken.
///
/// A committed-state reader takes one **before** it reads state and hands it back together
/// with the copy it read; [`DataContractCache::insert_committed`] drops the copy if a block
/// was committed in between, because the copy may then predate a rewrite that block made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommittedGeneration(u64);

/// DataContract cache that handles both global and block data.
///
/// Two kinds of reader share this cache, and neither may be served the other's view of a
/// contract:
///
/// * **Block execution** reads through the block transaction, on the single consensus
///   thread. Its reads and rewrites land in the block cache, which is cleared when a block
///   starts ([`Self::clear_block_cache`]) and promoted into the global cache once the block
///   is committed ([`Self::merge_and_clear_block_cache`]).
/// * **Committed-state readers** (the query threads) read with no transaction, concurrently
///   with block execution, and populate the global cache with what they read. `check_tx`
///   threads read through a fresh transaction of their own and never write to the cache.
///
/// CONSENSUS-CRITICAL. Block execution serializes documents and validates transitions
/// against whatever contract definition this cache hands it. If two honest validators
/// resolve the same contract differently while executing the same block, they write
/// different bytes and produce different app hashes. Two mechanisms keep a transactional
/// read from ever resolving to a definition older than the one its transaction holds:
///
/// * [`Self::mark_modified_in_block`]: once the block transaction rewrites a contract, a
///   transactional read of it never falls back to the global cache. The global copy is
///   committed state, which a committed-state reader may legitimately have (re)inserted
///   after the rewrite, and which the transaction has moved past. A block-cache miss for
///   such a contract goes to state through the transaction instead.
/// * [`CommittedGeneration`]: a committed-state copy enters the global cache only if no block
///   was committed between the read that produced it and the insert. Without this, a query
///   thread that read a contract, was descheduled across the block's commit and promotion,
///   and inserted afterwards would clobber the promoted definition with the pre-block one.
pub struct DataContractCache {
    global_cache: Cache<[u8; 32], Arc<DataContractFetchInfo>>,
    block_cache: Cache<[u8; 32], Arc<DataContractFetchInfo>>,
    /// Contracts the block transaction rewrote since the block cache was last cleared.
    ///
    /// This is a record of what the transaction changed, not a cache: [`Self::clear`] leaves
    /// it alone. It is reset with the block cache at block start and drained at promotion.
    block_modified: RwLock<HashSet<[u8; 32]>>,
    /// Bumped once per promotion, that is once per committed block.
    committed_generation: AtomicU64,
}

impl DataContractCache {
    /// Create a new DataContract cache instance
    pub fn new(global_cache_max_capacity: u64, block_cache_max_capacity: u64) -> Self {
        Self {
            global_cache: Cache::new(global_cache_max_capacity),
            block_cache: Cache::new(block_cache_max_capacity),
            block_modified: RwLock::new(HashSet::new()),
            committed_generation: AtomicU64::new(0),
        }
    }

    /// The snapshot a committed-state reader must take **before** reading state, to pass to
    /// [`Self::insert_committed`] with what it read.
    pub fn committed_generation(&self) -> CommittedGeneration {
        CommittedGeneration(self.committed_generation.load(Ordering::SeqCst))
    }

    /// Inserts a contract read or rewritten through the block transaction into the block
    /// cache.
    ///
    /// Only the consensus thread writes the block cache, so its inserts are sequential; the
    /// insert is nevertheless skipped if the block cache already holds the same contract at
    /// a strictly higher version, as defense in depth. Contract versions increase strictly
    /// monotonically (the update transition enforces `new == old + 1`, token configuration
    /// updates and system contract migrations bump the version), so a lower version is never
    /// fresh information. Same-version inserts overwrite: re-inserting an identical contract
    /// with a freshly calculated fee is the normal cache-hit fee path.
    pub fn insert_block(&self, fetch_info: Arc<DataContractFetchInfo>) {
        let data_contract_id_bytes = fetch_info.contract.id().to_buffer();

        self.block_cache
            .entry(data_contract_id_bytes)
            .and_compute_with(|existing| match existing {
                Some(entry) if entry.value().contract.version() > fetch_info.contract.version() => {
                    Op::Nop
                }
                _ => Op::Put(Arc::clone(&fetch_info)),
            });
    }

    /// Inserts a contract read from committed state into the global cache, unless a block
    /// was committed since `observed` was taken.
    ///
    /// The generation is compared inside moka's per-key compute closure, so there is no
    /// window between the comparison and the write: a promotion that bumps the generation
    /// either happens-before this insert, in which case the insert is dropped, or after it,
    /// in which case the promotion overwrites what was inserted. The same-contract version
    /// guard of [`Self::insert_block`] applies as well.
    pub fn insert_committed(
        &self,
        fetch_info: Arc<DataContractFetchInfo>,
        observed: CommittedGeneration,
    ) {
        let data_contract_id_bytes = fetch_info.contract.id().to_buffer();

        self.global_cache
            .entry(data_contract_id_bytes)
            .and_compute_with(|existing| {
                if self.committed_generation.load(Ordering::SeqCst) != observed.0 {
                    return Op::Nop;
                }
                match existing {
                    Some(entry)
                        if entry.value().contract.version() > fetch_info.contract.version() =>
                    {
                        Op::Nop
                    }
                    _ => Op::Put(Arc::clone(&fetch_info)),
                }
            });
    }

    /// Seeds the copy a rewrite of the contract in state produced.
    ///
    /// When the rewrite went through the block transaction (`in_block_transaction`), the
    /// contract is marked as modified in the block and the copy goes to the block cache, see
    /// [`Self::mark_modified_in_block`]. Otherwise the rewrite is committed already and the
    /// copy goes to the global cache through [`Self::insert_committed`]; `observed` must have
    /// been taken before the read that produced `fetch_info`.
    pub fn insert_rewritten(
        &self,
        fetch_info: Arc<DataContractFetchInfo>,
        in_block_transaction: bool,
        observed: CommittedGeneration,
    ) {
        if in_block_transaction {
            self.mark_modified_in_block(fetch_info.contract.id().to_buffer());
            self.insert_block(fetch_info);
        } else {
            self.insert_committed(fetch_info, observed);
        }
    }

    /// Tries to get a data contract from the block cache if the read is transactional, then
    /// from the global cache.
    ///
    /// A transactional read of a contract the block transaction rewrote
    /// ([`Self::mark_modified_in_block`]) does not fall back to the global cache: on a
    /// block-cache miss it returns `None`, and the caller reads state through the
    /// transaction. A read with no transaction only consults the global cache.
    pub fn get(
        &self,
        contract_id: [u8; 32],
        is_block_cache: bool,
    ) -> Option<Arc<DataContractFetchInfo>> {
        if is_block_cache {
            if let Some(fetch_info) = self.block_cache.get(&contract_id) {
                return Some(fetch_info);
            }
            if self.block_modified.read().contains(&contract_id) {
                return None;
            }
        }

        self.global_cache.get(&contract_id)
    }

    /// Remove contract from both block and global cache
    pub fn remove(&self, contract_id: [u8; 32]) {
        self.block_cache.remove(&contract_id);
        self.global_cache.remove(&contract_id);
    }

    /// Records that the block transaction rewrote `contract_id`.
    ///
    /// From now until the block cache is cleared or promoted, a transactional read of this
    /// contract is served from the block cache or from state through the transaction, never
    /// from the global cache. Callers seed the post-write copy with [`Self::insert_block`]
    /// so that the next read is a hit; the mark is what makes an eviction or a rollback safe.
    pub fn mark_modified_in_block(&self, contract_id: [u8; 32]) {
        self.block_modified.write().insert(contract_id);
    }

    /// Whether the block transaction rewrote `contract_id` since the block cache was cleared.
    pub fn is_modified_in_block(&self, contract_id: [u8; 32]) -> bool {
        self.block_modified.read().contains(&contract_id)
    }

    /// Drops from the block cache every contract the block transaction rewrote, keeping the
    /// contracts marked as modified.
    ///
    /// For after a savepoint rollback: the rollback reverted the rewrites in state, but the
    /// block cache still holds the post-write copies seeded when they were applied. Dropping
    /// them makes the next transactional read go to state, which now holds whatever the
    /// rollback restored. Entries the block only read stay: nothing rewrote what they hold.
    pub fn drop_block_modified_entries(&self) {
        for contract_id in self.block_modified.read().iter() {
            self.block_cache.remove(contract_id);
        }
    }

    /// Promotes the block cache into the global cache and clears it.
    ///
    /// Call this once the block transaction is **committed**, never before: from this call
    /// on, committed-state readers are served the block's definitions, and a reader that
    /// took its [`CommittedGeneration`] snapshot before this call can no longer insert. Both
    /// are only correct once state itself holds what the block wrote.
    ///
    /// Promotion is unconditional: everything the block read or rewrote through its
    /// transaction is committed state now, and nothing else could have written state in the
    /// meantime. A rewritten contract that is no longer in the block cache (evicted, or
    /// dropped by a rollback and not read again) is removed from the global cache instead,
    /// because a committed-state reader may have inserted the pre-block definition there
    /// while the block was executing.
    pub fn merge_and_clear_block_cache(&self) {
        // Bumping first closes the door on committed-state readers that read before the
        // commit: their inserts are dropped from here on, and any that already landed are
        // overwritten or removed below.
        self.committed_generation.fetch_add(1, Ordering::SeqCst);

        let modified = std::mem::take(&mut *self.block_modified.write());
        for contract_id in modified {
            if !self.block_cache.contains_key(&contract_id) {
                self.global_cache.remove(&contract_id);
            }
        }

        for (contract_id, fetch_info) in self.block_cache.iter() {
            self.global_cache
                .insert(Arc::unwrap_or_clone(contract_id), fetch_info);
        }
        self.block_cache.invalidate_all();
    }

    /// Clears the block cache and the record of what the block transaction rewrote.
    ///
    /// For the start of a block: a fresh transaction sees exactly committed state, which is
    /// what the global cache mirrors, so nothing needs protecting yet.
    pub fn clear_block_cache(&self) {
        self.block_cache.invalidate_all();
        self.block_modified.write().clear();
    }

    /// Drops every cached entry from both caches.
    ///
    /// The record of what the block transaction rewrote is kept: it is not a cache, and a
    /// clear in the middle of a block (a migration that rewrote contracts in state and wants
    /// every reader to reload them) must not let transactional reads fall back to the
    /// global cache again.
    pub fn clear(&self) {
        self.block_cache.invalidate_all();
        self.global_cache.invalidate_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::fee::fee_result::FeeResult;
    use dpp::version::PlatformVersion;

    /// Two copies of the SAME contract (same id) at the given versions. The fixture generates
    /// a fresh contract id per call, so both copies must derive from a single fixture.
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

    mod get {
        use super::*;

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

        /// The report's interleaving at the cache level: the block rewrote the contract and
        /// the block cache no longer holds it, while a committed-state reader has since put
        /// the committed (pre-rewrite) copy into the global cache. A transactional read must
        /// miss, so that the caller reads state through the transaction, rather than be
        /// handed the committed copy.
        #[test]
        fn test_get_in_block_does_not_fall_back_to_global_for_a_modified_contract() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (committed, _) = same_contract_at_versions(1, 2);
            let contract_id = committed.contract.id().to_buffer();

            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache
                .insert_committed(committed, data_contract_cache.committed_generation());

            assert!(
                data_contract_cache.get(contract_id, true).is_none(),
                "a transactional read of a rewritten contract must not be served the committed copy"
            );
        }

        #[test]
        fn test_get_in_block_reads_block_cache_for_a_modified_contract() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (committed, rewritten) = same_contract_at_versions(1, 2);
            let contract_id = committed.contract.id().to_buffer();

            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache.insert_block(rewritten);
            data_contract_cache
                .insert_committed(committed, data_contract_cache.committed_generation());

            let cached = data_contract_cache
                .get(contract_id, true)
                .expect("should be present");
            assert_eq!(cached.contract.version(), 2);
        }

        /// Committed-state readers are unaffected by the block's rewrite until it commits.
        #[test]
        fn test_get_committed_still_reads_global_for_a_modified_contract() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (committed, rewritten) = same_contract_at_versions(1, 2);
            let contract_id = committed.contract.id().to_buffer();

            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache.insert_block(rewritten);
            data_contract_cache
                .insert_committed(committed, data_contract_cache.committed_generation());

            let cached = data_contract_cache
                .get(contract_id, false)
                .expect("should be present");
            assert_eq!(cached.contract.version(), 1);
        }
    }

    mod insert_block {
        use super::*;

        /// A lower version must not clobber a higher one already in the block cache.
        #[test]
        fn test_insert_does_not_overwrite_newer_version_with_older() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (stale, newer) = same_contract_at_versions(1, 2);
            let contract_id = newer.contract.id().to_buffer();
            data_contract_cache.insert_block(newer);

            data_contract_cache.insert_block(stale);

            let cached = data_contract_cache
                .get(contract_id, true)
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
            data_contract_cache.insert_block(original);

            Arc::make_mut(&mut with_fee).fee = Some(FeeResult::new_from_processing_fee(1));
            data_contract_cache.insert_block(with_fee);

            let cached = data_contract_cache
                .get(contract_id, true)
                .expect("should be present");
            assert!(cached.fee.is_some(), "same-version insert must overwrite");
        }
    }

    mod insert_committed {
        use super::*;

        #[test]
        fn test_insert_with_a_current_snapshot_is_stored() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (fetch_info, _) = same_contract_at_versions(1, 1);
            let contract_id = fetch_info.contract.id().to_buffer();

            let observed = data_contract_cache.committed_generation();
            data_contract_cache.insert_committed(fetch_info, observed);

            assert!(data_contract_cache.get(contract_id, false).is_some());
        }

        /// A delayed insert carrying an older contract version must not clobber a newer
        /// entry, even when the reader's snapshot is current (nothing committed in between
        /// and the newer copy was inserted by another committed-state reader).
        #[test]
        fn test_insert_does_not_overwrite_newer_version_with_older() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (stale, newer) = same_contract_at_versions(1, 2);
            let contract_id = newer.contract.id().to_buffer();
            let observed = data_contract_cache.committed_generation();
            data_contract_cache.insert_committed(newer, observed);

            data_contract_cache.insert_committed(stale, observed);

            let cached = data_contract_cache
                .get(contract_id, false)
                .expect("should be present");
            assert_eq!(cached.contract.version(), 2);
        }

        /// The full race, end to end: a query thread snapshots the generation and reads the
        /// pre-block contract from committed state, block execution rewrites the contract
        /// and seeds the block cache, the block commits and promotes, and only then does the
        /// query thread perform its insert. The promoted contract must survive.
        #[test]
        fn test_insert_with_a_snapshot_taken_before_a_promotion_is_dropped() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (stale, rewritten) = same_contract_at_versions(1, 2);
            let contract_id = stale.contract.id().to_buffer();

            // The query thread snapshots and reads committed state, then is descheduled.
            let observed_before_the_block = data_contract_cache.committed_generation();

            // Block execution rewrites the contract; the block commits and promotes.
            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache.insert_block(rewritten);
            data_contract_cache.merge_and_clear_block_cache();

            // The query thread wakes up and performs its stale insert.
            data_contract_cache.insert_committed(stale, observed_before_the_block);

            let cached = data_contract_cache
                .get(contract_id, false)
                .expect("should be present");
            assert_eq!(
                cached.contract.version(),
                2,
                "the promoted contract must survive a delayed stale insert"
            );
        }

        /// The same delayed insert when the rewrite did not bump the contract version (an
        /// in-place migration such as the schema property strip) and the promoted entry is
        /// no longer resident. The version guard has nothing to compare against here; only
        /// the generation check stops the stale copy from sticking.
        #[test]
        fn test_same_version_insert_with_a_snapshot_taken_before_a_promotion_is_dropped() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (mut stale, mut rewritten) = same_contract_at_versions(1, 1);
            let contract_id = stale.contract.id().to_buffer();
            Arc::make_mut(&mut stale).fee = None;
            Arc::make_mut(&mut rewritten).fee = Some(FeeResult::new_from_processing_fee(1));

            let observed_before_the_block = data_contract_cache.committed_generation();

            // The block rewrote the contract but nothing read it afterwards, so the block
            // cache has no copy to promote and the global entry is removed instead.
            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache.merge_and_clear_block_cache();

            data_contract_cache.insert_committed(stale, observed_before_the_block);

            assert!(
                data_contract_cache.get(contract_id, false).is_none(),
                "a committed-state copy read before the commit must not enter the global cache after it"
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

            data_contract_cache
                .insert_committed(fetch_info, data_contract_cache.committed_generation());
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

            data_contract_cache.insert_committed(
                fetch_info_global,
                data_contract_cache.committed_generation(),
            );
            data_contract_cache.insert_block(fetch_info_block);
            data_contract_cache.remove(contract_id);

            assert!(data_contract_cache.block_cache.get(&contract_id).is_none());
            assert!(data_contract_cache.global_cache.get(&contract_id).is_none());
        }
    }

    mod drop_block_modified_entries {
        use super::*;

        /// After a rollback the post-write copies must go, but the contracts stay marked so
        /// that the next transactional read still bypasses the global cache.
        #[test]
        fn test_drops_modified_entries_from_block_cache_and_keeps_them_marked() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (committed, rewritten) = same_contract_at_versions(1, 2);
            let contract_id = committed.contract.id().to_buffer();

            data_contract_cache
                .insert_committed(committed, data_contract_cache.committed_generation());
            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache.insert_block(rewritten);

            data_contract_cache.drop_block_modified_entries();

            assert!(data_contract_cache.block_cache.get(&contract_id).is_none());
            assert!(data_contract_cache.is_modified_in_block(contract_id));
            assert!(
                data_contract_cache.get(contract_id, true).is_none(),
                "the transactional read must go to state, not to the committed copy"
            );
        }

        #[test]
        fn test_leaves_unmodified_block_entries_in_place() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (only_read, _) = same_contract_at_versions(1, 1);
            let contract_id = only_read.contract.id().to_buffer();
            data_contract_cache.insert_block(only_read);

            data_contract_cache.drop_block_modified_entries();

            assert!(data_contract_cache.get(contract_id, true).is_some());
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

            data_contract_cache.insert_block(fetch_info);
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

            data_contract_cache.insert_block(fetch_info);
            data_contract_cache.merge_and_clear_block_cache();

            assert!(data_contract_cache.block_cache.get(&contract_id).is_none());
        }

        /// A committed-state reader put the pre-block copy into the global cache while the
        /// block was executing. Promotion must replace it with what the block wrote.
        #[test]
        fn test_merge_promotes_a_rewritten_contract_over_the_committed_copy() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (committed, rewritten) = same_contract_at_versions(1, 2);
            let contract_id = committed.contract.id().to_buffer();

            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache.insert_block(rewritten);
            data_contract_cache
                .insert_committed(committed, data_contract_cache.committed_generation());

            data_contract_cache.merge_and_clear_block_cache();

            let cached = data_contract_cache
                .get(contract_id, false)
                .expect("should be present");
            assert_eq!(cached.contract.version(), 2);
        }

        /// Same, but the block cache no longer holds the rewritten copy (evicted, or dropped
        /// by a rollback and not read again). There is nothing to promote, so the committed
        /// copy a reader inserted during the block must be removed: it is stale now.
        #[test]
        fn test_merge_removes_the_committed_copy_of_a_rewritten_contract_missing_from_the_block_cache(
        ) {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (committed, _) = same_contract_at_versions(1, 2);
            let contract_id = committed.contract.id().to_buffer();

            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache
                .insert_committed(committed, data_contract_cache.committed_generation());

            data_contract_cache.merge_and_clear_block_cache();

            assert!(data_contract_cache.get(contract_id, false).is_none());
        }

        #[test]
        fn test_merge_clears_modified_marks() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (_, rewritten) = same_contract_at_versions(1, 2);
            let contract_id = rewritten.contract.id().to_buffer();

            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache.insert_block(rewritten);

            data_contract_cache.merge_and_clear_block_cache();

            assert!(!data_contract_cache.is_modified_in_block(contract_id));
        }

        #[test]
        fn test_merge_advances_the_committed_generation() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let before = data_contract_cache.committed_generation();
            data_contract_cache.merge_and_clear_block_cache();

            assert_ne!(before, data_contract_cache.committed_generation());
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

            data_contract_cache.insert_committed(
                fetch_info_global,
                data_contract_cache.committed_generation(),
            );
            data_contract_cache.insert_block(fetch_info_block);
            data_contract_cache.clear();

            assert!(data_contract_cache.get(contract_id, false).is_none());
            assert!(data_contract_cache.block_cache.get(&contract_id).is_none());
        }

        /// A mid-block clear (the schema property strip migration) must not let
        /// transactional reads of the rewritten contracts fall back to the global cache.
        #[test]
        fn test_clear_keeps_modified_marks() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (_, rewritten) = same_contract_at_versions(1, 2);
            let contract_id = rewritten.contract.id().to_buffer();

            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache.insert_block(rewritten);
            data_contract_cache.clear();

            assert!(data_contract_cache.is_modified_in_block(contract_id));
        }

        #[test]
        fn test_clear_block_cache_clears_modified_marks() {
            let data_contract_cache = DataContractCache::new(10, 10);

            let (_, rewritten) = same_contract_at_versions(1, 2);
            let contract_id = rewritten.contract.id().to_buffer();

            data_contract_cache.mark_modified_in_block(contract_id);
            data_contract_cache.insert_block(rewritten);
            data_contract_cache.clear_block_cache();

            assert!(!data_contract_cache.is_modified_in_block(contract_id));
            assert!(data_contract_cache.block_cache.get(&contract_id).is_none());
        }
    }
}
