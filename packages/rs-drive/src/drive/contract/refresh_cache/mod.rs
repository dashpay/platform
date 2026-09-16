use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Replaces whatever the data contract cache holds for `contract_id` with what state holds
    /// for it right now, reading through `transaction`.
    ///
    /// CONSENSUS-CRITICAL. Every path that rewrites a contract in state ends here: the drive
    /// operation batch runs it as the `ApplyContract` finalization task once the batch is
    /// applied, and the protocol upgrade migrations call it after rewriting a system contract
    /// outside the batch. Without it a node that had read the contract once would keep
    /// serializing documents against its cached copy while a node with a cold cache reads the
    /// rewritten one from state, and the two write different bytes for the same transition.
    ///
    /// Three details are load-bearing:
    ///
    /// * The read **bypasses both caches**. Going through `get_contract_with_fetch_info` would
    ///   consult the cache first and hand back the very entry being replaced.
    /// * With a transaction, the contract is **marked as modified in the block** and the result
    ///   is seeded into the **block** cache. The write is still uncommitted at that point: the
    ///   block cache is the first cache a transactional read consults, it is promoted to the
    ///   global cache once the block commits (`merge_and_clear_block_cache`), and it is dropped
    ///   if the block never does. Seeding the global cache instead would publish an uncommitted
    ///   definition to every reader and survive a rejected block. The mark is what closes the
    ///   race this helper exists for. The superseded copy is dropped from both caches here, but
    ///   a read-only query thread, which reads committed state with no transaction and
    ///   populates the global cache, can put the committed (pre-write) definition straight back
    ///   into the global cache; a transactional read of a marked contract never falls back to
    ///   the global cache, so whether that happens is immaterial. The other half of the race, a
    ///   query thread that read the pre-write contract, was descheduled, and performs its
    ///   insert only after the block was committed and promoted, is closed by the committed
    ///   generation check in [`DataContractCache::insert_committed`].
    /// * Without a transaction the write is committed already: the superseded copy is evicted
    ///   from both caches through [`DataContractCache::replace_committed`], which also advances
    ///   the committed generation so that a reader that read the pre-write contract cannot put
    ///   it back, and the next reader reloads the contract from state. Block execution never
    ///   takes this path; it exists for direct callers that write outside a block.
    ///
    ///   [`DataContractCache::replace_committed`]: crate::cache::DataContractCache::replace_committed
    ///
    ///   [`DataContractCache::insert_committed`]: crate::cache::DataContractCache::insert_committed
    ///
    /// The caller must invoke this *after* the block cache has been cleared for the block
    /// (`clear_drive_block_cache`), or the seed is wiped before anything reads it.
    ///
    /// Billing is unaffected: `fetch_contract_v0` computes its `OperationCost` with grovedb
    /// value caching disabled precisely so a contract fetch costs the same every time, and
    /// derives a fee from that cost only when an epoch is supplied; a cache hit seeded here
    /// therefore bills exactly like the cold fetch it replaces.
    pub fn refresh_data_contract_cache_from_state(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Without a transaction the rewrite is committed already: evicting is enough for the
        // next reader, which reloads the contract from state, and recording the commit keeps
        // a reader that read the pre-write contract from putting it back.
        if transaction.is_none() {
            self.cache
                .data_contracts
                .replace_committed(contract_id, None);
            return Ok(());
        }

        let maybe_fetch_info = self.fetch_contract_and_add_operations(
            contract_id,
            None,
            transaction,
            &mut vec![],
            platform_version,
        )?;

        // From here on a transactional read of this contract is served from the block cache
        // or from state through the transaction, never from a copy a committed-state reader
        // puts into the global cache.
        self.cache
            .data_contracts
            .mark_modified_in_block(contract_id);

        // Drop the superseded copy from both caches before re-seeding.
        self.cache.data_contracts.remove(contract_id);

        // A contract that is not in state has nothing to cache: leaving both caches empty is
        // then the correct outcome, and the next reader will fetch and find it absent.
        if let Some(fetch_info) = maybe_fetch_info {
            self.cache.data_contracts.insert_block(fetch_info);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::contract::tests::setup_reference_contract;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
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
                BlockInfo::default(),
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
        drive.cache.data_contracts.insert_committed(
            Arc::new(DataContractFetchInfo {
                contract: stale,
                storage_flags: None,
                cost: Default::default(),
                fee: None,
            }),
            drive.cache.data_contracts.committed_generation(),
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
                BlockInfo::default(),
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
        assert!(
            drive.cache.data_contracts.is_modified_in_block(contract_id),
            "the rewrite must be recorded so transactional reads never fall back to the global cache"
        );
    }

    /// The race the refresh exists for, at the Drive level: the block rewrote the contract,
    /// a concurrent committed-state query put the committed (pre-write) definition back into
    /// the global cache, and the block cache no longer holds the seeded copy. A transactional
    /// read must still resolve to what the transaction holds.
    #[test]
    fn should_not_serve_a_transactional_read_the_committed_copy_a_query_put_back() {
        let (drive, contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();
        let contract_id = contract.id().to_buffer();

        let transaction = drive.grove.start_transaction();

        // The block rewrites the contract. `apply_contract` does not touch the cache on its
        // own, which is exactly the situation the refresh is for.
        let mut rewritten = contract.clone();
        rewritten.increment_version();
        drive
            .apply_contract(
                &rewritten,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to rewrite the contract");
        drive
            .refresh_data_contract_cache_from_state(
                contract_id,
                Some(&transaction),
                platform_version,
            )
            .expect("expected the refresh to succeed");

        // The concurrent query: reads committed state (still the pre-write contract) and
        // populates the global cache with it.
        let committed = drive
            .get_contract_with_fetch_info(contract_id, true, None, platform_version)
            .expect("expected the query to succeed")
            .expect("expected the committed contract");
        assert_eq!(
            committed.contract.version(),
            1,
            "precondition: the query read the pre-write contract"
        );
        assert_eq!(
            drive
                .cache
                .data_contracts
                .get(contract_id, false)
                .expect("the query must have populated the global cache")
                .contract
                .version(),
            1
        );

        // With the seed resident, the block reads it.
        let in_block = drive
            .get_contract_with_fetch_info(contract_id, true, Some(&transaction), platform_version)
            .expect("expected the transactional read to succeed")
            .expect("expected the contract");
        assert_eq!(in_block.contract.version(), 2);

        // Without the seed (evicted, or dropped by a rollback), the block must go to state,
        // never to the committed copy the query left in the global cache.
        drive.cache.data_contracts.drop_block_modified_entries();
        let in_block = drive
            .get_contract_with_fetch_info(contract_id, true, Some(&transaction), platform_version)
            .expect("expected the transactional read to succeed")
            .expect("expected the contract");
        assert_eq!(
            in_block.contract.version(),
            2,
            "a transactional read of a rewritten contract must not be served the committed copy"
        );
    }

    /// A refresh outside any transaction is a committed rewrite: it evicts the superseded copy,
    /// the next reader reloads the contract from state, and nothing is recorded about the block.
    #[test]
    fn should_evict_the_cached_copy_for_a_committed_rewrite() {
        let (drive, contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();
        let contract_id = contract.id().to_buffer();

        drive
            .get_contract_with_fetch_info(contract_id, true, None, platform_version)
            .expect("expected the read to succeed")
            .expect("expected the contract");

        let mut rewritten = contract.clone();
        rewritten.increment_version();
        drive
            .apply_contract(
                &rewritten,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to rewrite the contract");
        // A committed-state reader that read the pre-write contract before the rewrite and
        // performs its insert only now.
        let observed_before_the_rewrite = drive.cache.data_contracts.committed_generation();
        let pre_write = drive
            .cache
            .data_contracts
            .get(contract_id, false)
            .expect("the warmed copy must be present");

        drive
            .refresh_data_contract_cache_from_state(contract_id, None, platform_version)
            .expect("expected the refresh to succeed");

        assert!(
            drive.cache.data_contracts.get(contract_id, false).is_none(),
            "the superseded copy must be evicted"
        );
        assert!(!drive.cache.data_contracts.is_modified_in_block(contract_id));

        drive
            .cache
            .data_contracts
            .insert_committed(pre_write, observed_before_the_rewrite);
        assert!(
            drive.cache.data_contracts.get(contract_id, false).is_none(),
            "a reader that read the pre-write contract must not put it back after the rewrite"
        );
        assert_eq!(
            drive
                .get_contract_with_fetch_info(contract_id, true, None, platform_version)
                .expect("expected the read to succeed")
                .expect("expected the contract")
                .contract
                .version(),
            2,
            "the next reader must reload the rewritten contract from state"
        );
    }
}
