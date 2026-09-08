use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Replaces whatever the data contract cache holds for `contract_id` with what state holds
    /// for it right now, reading through `transaction`.
    ///
    /// CONSENSUS-CRITICAL. A write that reaches state outside the drive operation batch — as
    /// the protocol upgrade migrations do, via `insert_contract`/`apply_contract` — carries no
    /// `RemoveDataContractFromCache` finalization task, so nothing evicts the superseded
    /// definition. A node that had read the contract once would keep serializing documents
    /// against its cached copy while a node with a cold cache reads the rewritten one from
    /// state, and the two write different bytes for the same transition.
    ///
    /// Two details are load-bearing:
    ///
    /// * The read **bypasses both caches**. Going through `get_contract_with_fetch_info` would
    ///   consult the cache first and hand back the very entry being replaced; worse, a
    ///   read-only query thread — which reads committed state with no transaction and does
    ///   populate the global cache — could race a pre-write copy back in between the eviction
    ///   and the re-seed. The other half of that race — a query thread that read the
    ///   pre-migration contract, was descheduled, and performs its cache insert only after the
    ///   migrated contract was promoted to the global cache — is closed by the monotonic
    ///   version guard in [`DataContractCache::insert`], which requires every migration rewrite
    ///   to bump the contract's version (the v13 DPNS rewrite goes 1 → 2, the v14 DashPay
    ///   rewrite likewise).
    ///
    ///   [`DataContractCache::insert`]: crate::cache::DataContractCache::insert
    /// * The result is seeded into the **block** cache whenever a transaction is supplied. The
    ///   write is still uncommitted at that point: the block cache is the first cache a
    ///   transactional read consults, it is promoted to the global cache when the block commits
    ///   (`merge_and_clear_block_cache`), and it is dropped if the block never does. Seeding
    ///   the global cache instead would publish an uncommitted definition to every reader and
    ///   survive a rejected block.
    ///
    /// The caller must invoke this *after* the block cache has been cleared for the block
    /// (`clear_drive_block_cache`), or the seed is wiped before anything reads it.
    ///
    /// Billing is unaffected: `fetch_contract_v0` computes its `OperationCost` with grovedb
    /// value caching disabled precisely so a contract fetch costs the same every time, and
    /// derives a fee from that cost only when an epoch is supplied — so a cache hit seeded here
    /// bills exactly like the cold fetch it replaces.
    pub fn refresh_data_contract_cache_from_state(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let maybe_fetch_info = self.fetch_contract_and_add_operations(
            contract_id,
            None,
            transaction,
            &mut vec![],
            platform_version,
        )?;

        // Drop the superseded copy from both caches before re-seeding.
        self.cache.data_contracts.remove(contract_id);

        // A contract that is not in state has nothing to cache — leaving both caches empty is
        // then the correct outcome, and the next reader will fetch and find it absent.
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
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use std::sync::Arc;

    /// A cached copy that no longer matches state must be replaced, so that a warm reader and
    /// a cold reader resolve the same contract.
    #[test]
    fn should_replace_a_cached_copy_that_no_longer_matches_state() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let dpns = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS");
        let contract_id = dpns.id().to_buffer();

        drive
            .apply_contract(
                &dpns,
                dpp::block::block_info::BlockInfo::default(),
                true,
                None,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to store DPNS");

        // Stand in for the pre-write copy a long-lived node carries: cached, but not what
        // state holds.
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
            .refresh_data_contract_cache_from_state(
                contract_id,
                Some(&transaction),
                platform_version,
            )
            .expect("expected the refresh to succeed");

        let resolved = drive
            .get_contract_with_fetch_info(contract_id, false, Some(&transaction), platform_version)
            .expect("expected to resolve the contract")
            .expect("expected the contract to be present");

        assert_eq!(
            resolved.contract.version(),
            dpns.version(),
            "the refreshed cache must hand back what state holds, not the stale copy"
        );
    }

    /// The refresh seeds the block cache, not the global one, so an uncommitted rewrite is
    /// never published to readers that are not part of the block.
    #[test]
    fn should_seed_the_block_cache_and_leave_the_global_cache_empty() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let dpns = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS");
        let contract_id = dpns.id().to_buffer();

        drive
            .apply_contract(
                &dpns,
                dpp::block::block_info::BlockInfo::default(),
                true,
                None,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to store DPNS");

        drive
            .refresh_data_contract_cache_from_state(
                contract_id,
                Some(&transaction),
                platform_version,
            )
            .expect("expected the refresh to succeed");

        assert!(
            drive.cache.data_contracts.get(contract_id, true).is_some(),
            "a transactional reader must see the refreshed contract"
        );
        assert!(
            drive.cache.data_contracts.get(contract_id, false).is_none(),
            "a committed-state reader must not see the uncommitted rewrite"
        );
    }
}
