use crate::drive::shielded::nullifiers::queries::{
    shielded_recent_nullifiers_path, SHIELDED_RECENT_NULLIFIERS_KEY_U8,
};
use crate::drive::shielded::nullifiers::types::CompactedNullifiers;
use crate::drive::shielded::paths::shielded_credit_pool_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use grovedb::Element;
use grovedb::TransactionArg;

use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation of storing nullifiers for a block.
    ///
    /// Serializes the nullifier list using bincode and stores it in the
    /// shielded credit pool per-block nullifiers count sum tree keyed by block height.
    /// Each entry is an ItemWithSumItem where:
    /// - The item contains the serialized nullifiers
    /// - The sum value is the number of nullifiers
    ///
    /// Before storing, checks if compaction thresholds are exceeded and triggers
    /// compaction if necessary. If compaction occurs, the current block's nullifiers
    /// are included in the compaction rather than stored separately.
    pub(in crate::drive) fn store_nullifiers_for_block_v0(
        &self,
        nullifiers: &[[u8; 32]],
        block_height: u64,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Early return if there are no nullifiers to store
        if nullifiers.is_empty() {
            return Ok(());
        }

        // Check if compaction is needed - if so, include current nullifiers in compaction
        let compacted = self.check_and_compact_nullifiers_if_needed(
            nullifiers,
            block_height,
            block_time_ms,
            transaction,
            platform_version,
        )?;

        // If we compacted, the current nullifiers are already included - don't store separately
        if compacted {
            return Ok(());
        }

        // Serialize the nullifiers using CompactedNullifiers wrapper
        let serialized = CompactedNullifiers::new(nullifiers.to_vec()).encode()?;

        // The sum value is the number of nullifiers
        let entry_count = i64::try_from(nullifiers.len()).map_err(|_| {
            Error::Drive(crate::error::drive::DriveError::CorruptedDriveState(
                "nullifier count exceeds i64::MAX".to_string(),
            ))
        })?;

        // Store in the shielded pool per-block nullifiers count sum tree with block height as key
        let path = shielded_recent_nullifiers_path();

        // Use block height as the key (big-endian for proper ordering)
        let key = block_height.to_be_bytes();

        // Insert as ItemWithSumItem where:
        // - item data = serialized nullifiers
        // - sum value = number of nullifiers
        let mut drive_operations = vec![];
        self.grove_insert(
            path.as_ref().into(),
            &key,
            Element::new_item_with_sum_item(serialized, entry_count),
            transaction,
            None,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // Apply any operations that were generated
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            drive_operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(())
    }

    /// Checks if compaction thresholds are exceeded and triggers compaction if needed.
    /// If compaction occurs, the provided nullifiers are included in the compaction.
    ///
    /// Returns true if compaction was performed (meaning the nullifiers were included).
    fn check_and_compact_nullifiers_if_needed(
        &self,
        nullifiers: &[[u8; 32]],
        block_height: u64,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let pool_path = shielded_credit_pool_path();

        // Get the count sum tree element to check current count and sum
        let mut drive_operations = vec![];
        let tree_element = self.grove_get_raw(
            (&pool_path).into(),
            &[SHIELDED_RECENT_NULLIFIERS_KEY_U8],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // The recent-nullifiers subtree is wrapped in `Element::NotSummed` so
        // that its sum side does not propagate into the enclosing shielded
        // pool SumTree. `underlying()` peels the wrapper so we can read the
        // inner CountSumTree's own (count, sum) for the compaction trigger.
        if let Some(Element::CountSumTree(_, count, sum, _)) =
            tree_element.as_ref().map(|e| e.underlying())
        {
            let max_blocks = platform_version
                .drive
                .methods
                .saved_block_transactions
                .max_blocks_before_nullifier_compaction as u64;
            let max_nullifiers = platform_version
                .drive
                .methods
                .saved_block_transactions
                .max_nullifiers_before_compaction as i64;

            // Check if either threshold would be exceeded after adding the current block
            // count + 1 for the new block, sum + current nullifiers count
            let new_count = *count + 1;
            let new_sum = *sum + nullifiers.len() as i64;

            if new_count >= max_blocks || new_sum >= max_nullifiers {
                // Trigger compaction, including the current block's nullifiers
                self.compact_nullifiers_with_current_block(
                    nullifiers,
                    block_height,
                    block_time_ms,
                    transaction,
                    platform_version,
                )?;
                return Ok(true);
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::nullifiers::queries::{
        shielded_compacted_nullifiers_path_vec, shielded_recent_nullifiers_path_vec,
    };
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::{PathQuery, Query, SizedQuery};

    /// Storing an empty nullifier list must be a no-op: nothing is written,
    /// compaction isn't triggered, and no error is returned.
    #[test]
    fn store_empty_nullifiers_is_noop() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        drive
            .store_nullifiers_for_block_v0(&[], 10, 1_000, None, platform_version)
            .expect("empty store should succeed");

        // Nothing should be stored in the recent-nullifiers tree.
        let path = shielded_recent_nullifiers_path_vec();
        let mut query = Query::new();
        query.insert_all();
        let pq = PathQuery::new(path, SizedQuery::new(query, None, None));
        let (results, _) = drive
            .grove_get_path_query(
                &pq,
                None,
                grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("query should succeed");
        assert_eq!(results.to_key_elements().len(), 0);
    }

    /// Storing a single batch of nullifiers for a block must round-trip via
    /// fetch_recent_nullifier_changes without triggering compaction.
    #[test]
    fn store_single_block_round_trips_via_fetch() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let nullifiers = vec![[1u8; 32], [2u8; 32], [3u8; 32]];

        drive
            .store_nullifiers_for_block_v0(&nullifiers, 42, 1_000, None, platform_version)
            .expect("store should succeed");

        let changes = drive
            .fetch_recent_nullifier_changes(0, None, None, platform_version)
            .expect("fetch should succeed");

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].block_height, 42);
        assert_eq!(changes[0].nullifiers.as_slice(), nullifiers.as_slice());

        // Nothing should have been compacted (we only stored one block).
        let compacted = drive
            .fetch_compacted_nullifier_changes(0, None, None, platform_version)
            .expect("fetch compacted should succeed");
        assert_eq!(compacted.len(), 0);
    }

    /// Exceeding the max_nullifiers_before_compaction threshold should trigger
    /// compaction: recent-nullifiers tree is drained, compacted tree has one entry.
    #[test]
    fn compaction_triggers_on_nullifier_threshold() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Derive the compaction threshold from the active platform version so
        // this test stays correct if the constant is bumped in a future
        // drive version.
        let max_nullifiers = platform_version
            .drive
            .methods
            .saved_block_transactions
            .max_nullifiers_before_compaction;

        let nullifiers: Vec<[u8; 32]> = (0..max_nullifiers)
            .map(|i| [(i & 0xff) as u8; 32])
            .collect();

        drive
            .store_nullifiers_for_block_v0(&nullifiers, 1, 1_000, None, platform_version)
            .expect("first store should succeed");

        // First store doesn't trigger because count=0 before, sum=0 before.
        // new_sum = 0 + max_nullifiers >= max_nullifiers, so compaction IS
        // triggered on the first block. That means recent is empty and
        // compacted has one entry spanning [1,1].
        let recent = drive
            .fetch_recent_nullifier_changes(0, None, None, platform_version)
            .expect("fetch recent");
        assert_eq!(
            recent.len(),
            0,
            "recent should be empty after threshold-triggered compaction"
        );

        let compacted = drive
            .fetch_compacted_nullifier_changes(0, None, None, platform_version)
            .expect("fetch compacted");
        assert_eq!(compacted.len(), 1);
        assert_eq!(compacted[0].start_block, 1);
        assert_eq!(compacted[0].end_block, 1);

        // The compacted entry key must exist in GroveDB.
        let path = shielded_compacted_nullifiers_path_vec();
        let mut query = Query::new();
        query.insert_all();
        let pq = PathQuery::new(path, SizedQuery::new(query, None, None));
        let (results, _) = drive
            .grove_get_path_query(
                &pq,
                None,
                grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("query compacted");
        assert_eq!(results.to_key_elements().len(), 1);
    }

    /// Storing under a transaction (Some(&tx)) should commit the nullifiers
    /// only after the transaction commits.
    #[test]
    fn store_within_transaction_commits_correctly() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        drive
            .store_nullifiers_for_block_v0(
                &[[9u8; 32]],
                5,
                1_000,
                Some(&transaction),
                platform_version,
            )
            .expect("store in tx");

        // Without committing, the non-transactional reader should see nothing.
        let changes_no_tx = drive
            .fetch_recent_nullifier_changes(0, None, None, platform_version)
            .expect("fetch without tx");
        assert_eq!(changes_no_tx.len(), 0);

        drive
            .commit_transaction(transaction, &platform_version.drive)
            .expect("commit");

        let changes = drive
            .fetch_recent_nullifier_changes(0, None, None, platform_version)
            .expect("fetch after commit");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].block_height, 5);
    }
}
