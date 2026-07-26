use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Re-reads a data contract from state and re-seeds the in-memory data contract cache
    /// with it. Call this from a protocol-upgrade migration for every contract the
    /// migration **rewrites** — i.e. any contract that existed before the migration and
    /// so may already sit in a node's cache. A contract the migration introduces for the
    /// first time needs no refresh: the cache holds no negative entries, so no node can
    /// have a stale copy of a contract that never existed.
    ///
    /// CONSENSUS-CRITICAL. Contracts written by a state transition go through the drive
    /// operation batch, whose `RemoveDataContractFromCache` finalization task evicts the
    /// superseded copy from the cache. Migrations write contracts directly
    /// (`insert_contract` / `apply_contract`) and bypass that machinery entirely, so
    /// without this call a node that already holds the pre-migration contract in its
    /// global cache keeps serving that copy for the rest of the process lifetime, while a
    /// node whose cache is cold (freshly restarted, or the entry was evicted under
    /// capacity pressure) reads the migrated one. The two nodes then serialize documents
    /// of that contract against different `DataContract`s — a difference that reaches
    /// state, because `DocumentV0::serialize` picks its serialization version from the
    /// contract's config version — and produce different app hashes from the same block.
    ///
    /// The refreshed contract is placed in the **block** cache, not the global cache: at
    /// this point the migration's write is still uncommitted, and the block cache is the
    /// only cache that transactional reads consult first. It is promoted to the global
    /// cache once the block commits (`merge_and_clear_block_cache`), and dropped at the
    /// start of the next block if the block never commits (`clear_block_cache`).
    ///
    /// The read deliberately bypasses both caches rather than going through
    /// `get_contract_with_fetch_info`: a concurrent read-only query thread (which reads
    /// committed state, with no transaction, and does populate the global cache) could
    /// otherwise race a pre-migration copy back into the global cache between the
    /// eviction and the re-seed.
    ///
    /// Billing is unaffected. `fetch_contract_v0` computes the cached `OperationCost`
    /// with grovedb value caching disabled precisely so the cost of a contract fetch is
    /// deterministic, and derives `fee` from that cost only when an epoch is supplied —
    /// so a cache hit seeded here bills identically to the cold fetch it replaces.
    pub fn refresh_data_contract_cache_from_state(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Cache-bypassing read: the contract exactly as state now holds it.
        let maybe_fetch_info = self.fetch_contract_and_add_operations(
            contract_id,
            None,
            transaction,
            &mut vec![],
            platform_version,
        )?;

        // Drop the pre-migration copy from both the block and the global cache.
        self.cache.data_contracts.remove(contract_id);

        // Re-seed with what state now holds. A migration always writes the contract it
        // asks us to refresh, so `None` here means the contract genuinely is not in
        // state; leaving the caches empty is then the correct outcome.
        if let Some(fetch_info) = maybe_fetch_info {
            self.cache
                .data_contracts
                .insert(fetch_info, transaction.is_some());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::contract::DataContractFetchInfo;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use std::sync::Arc;

    /// A contract written directly to state must replace a stale cached copy, so that a
    /// warm node and a cold node read the same contract afterwards.
    #[test]
    fn test_refresh_replaces_stale_cached_contract() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let dpns = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS");
        let contract_id = dpns.id().to_buffer();

        // Warm the global cache with a contract that does NOT match state: a distinct
        // version stands in for the pre-migration copy a long-lived node would hold.
        let mut stale = dpns.clone();
        stale.set_version(u32::MAX);
        drive.cache.data_contracts.insert(
            Arc::new(DataContractFetchInfo {
                contract: stale,
                storage_flags: None,
                cost: Default::default(),
                fee: None,
            }),
            false,
        );

        drive
            .apply_contract(
                &dpns,
                BlockInfo::default(),
                true,
                None,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to apply contract");

        // Without the refresh this still reads the stale copy out of the global cache.
        drive
            .refresh_data_contract_cache_from_state(
                contract_id,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to refresh cache");

        let refreshed = drive
            .get_contract_with_fetch_info(contract_id, false, Some(&transaction), platform_version)
            .expect("expected to fetch contract")
            .expect("expected the contract to be present");

        assert_eq!(refreshed.contract.version(), dpns.version());
    }

    /// The refresh must seed the block cache, not the global cache: the migration's write
    /// is still uncommitted, so a non-transactional reader must not see it yet.
    #[test]
    fn test_refresh_seeds_block_cache_not_global_cache() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let dpns = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS");
        let contract_id = dpns.id().to_buffer();

        drive
            .apply_contract(
                &dpns,
                BlockInfo::default(),
                true,
                None,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to apply contract");

        drive
            .refresh_data_contract_cache_from_state(
                contract_id,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to refresh cache");

        assert!(
            drive.cache.data_contracts.get(contract_id, true).is_some(),
            "transactional reads must see the refreshed contract"
        );
        assert!(
            drive.cache.data_contracts.get(contract_id, false).is_none(),
            "the uncommitted contract must not be visible in the global cache"
        );
    }
}
