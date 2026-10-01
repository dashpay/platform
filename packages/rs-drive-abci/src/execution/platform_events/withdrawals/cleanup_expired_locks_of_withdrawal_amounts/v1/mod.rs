use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;

use crate::error::execution::ExecutionError;
use dpp::version::PlatformVersion;
use drive::drive::identity::withdrawals::paths::{
    get_withdrawal_core_credit_pool_balances_path_vec,
    get_withdrawal_core_dated_credit_inflows_sum_tree_path_vec,
    get_withdrawal_credit_inflows_sum_tree_path_vec,
    get_withdrawal_pending_asset_lock_inflows_path,
    get_withdrawal_pending_asset_lock_inflows_path_vec,
    get_withdrawal_transactions_sum_tree_path_vec,
};
use drive::drive::identity::withdrawals::{
    core_dated_credit_inflow_key, PendingAssetLockCreditInflow,
};
use drive::error::drive::DriveError;
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::{Element, MaybeTree, PathQuery, Query, QueryItem, SizedQuery, Transaction};
use drive::util::grove_operations::BatchDeleteApplyType;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Version 1 differs from version 0 in also pruning the trees of the withdrawal limit that
    /// exist from protocol version 14, each with the same per-block limit:
    ///
    /// * the expired entries of the credit inflows sum tree, keyed like the reservations by
    ///   the block time they stop counting toward the daily withdrawal limit;
    /// * the Core-dated credit inflows whose Core height has been reached;
    /// * the recorded Core credit pool balances older than the band of window starts the
    ///   Core-anchored limit reads (`core_credit_pool_window_max_blocks` back);
    /// * the asset locks consumed before Core mined them that waited that many Core blocks
    ///   without Core mining them: they never count. In practice Core mines an InstantSend
    ///   locked transaction within a block or two, so this only bounds the tree.
    pub(super) fn cleanup_expired_locks_of_withdrawal_amounts_v1(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let limit = platform_version
            .drive_abci
            .withdrawal_constants
            .cleanup_expired_locks_of_withdrawal_amounts_limit;

        if limit == 0 {
            // No clean up
            return Ok(());
        }

        let mut batch_operations = vec![];

        for path in [
            get_withdrawal_transactions_sum_tree_path_vec(),
            get_withdrawal_credit_inflows_sum_tree_path_vec(),
        ] {
            let mut path_query = PathQuery::new_single_query_item(
                path,
                QueryItem::RangeTo(..block_info.time_ms.to_be_bytes().to_vec()),
            );

            path_query.query.limit = Some(limit);

            self.drive.batch_delete_items_in_path_query(
                &path_query,
                true,
                // we know that we are not deleting a subtree
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                },
                Some(transaction),
                &mut batch_operations,
                &platform_version.drive,
            )?;
        }

        let window_max_blocks = platform_version
            .system_limits
            .core_credit_pool_window_max_blocks
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "cleanup_expired_locks_of_withdrawal_amounts v1 requires system_limits.core_credit_pool_window_max_blocks",
            )))?;
        let chain_locked_height = block_info.core_height;

        // Keys sort by the Core height an entry stops counting at, then the time it was
        // recorded at: every key below (chain locked height + 1, 0) has stopped counting.
        let mut range_prunes = vec![(
            get_withdrawal_core_dated_credit_inflows_sum_tree_path_vec(),
            core_dated_credit_inflow_key(chain_locked_height.saturating_add(1), 0),
        )];
        // The Core-anchored limit never reads a balance older than its farthest window start.
        if let Some(oldest_read_height) = chain_locked_height.checked_sub(window_max_blocks) {
            range_prunes.push((
                get_withdrawal_core_credit_pool_balances_path_vec(),
                oldest_read_height.to_be_bytes().to_vec(),
            ));
        }

        for (path, first_kept_key) in range_prunes {
            let mut path_query =
                PathQuery::new_single_query_item(path, QueryItem::RangeTo(..first_kept_key));
            path_query.query.limit = Some(limit);

            self.drive.batch_delete_items_in_path_query(
                &path_query,
                true,
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                },
                Some(transaction),
                &mut batch_operations,
                &platform_version.drive,
            )?;
        }

        // Pending entries are keyed by transaction id, so their age is in the value: read a
        // bounded batch and drop the ones that waited a whole band of Core blocks.
        let mut pending_query = Query::new();
        pending_query.insert_all();
        let (pending, _) = self.drive.grove_get_raw_path_query(
            &PathQuery::new(
                get_withdrawal_pending_asset_lock_inflows_path_vec(),
                SizedQuery::new(pending_query, Some(limit), None),
            ),
            Some(transaction),
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;
        let pending_path = get_withdrawal_pending_asset_lock_inflows_path();
        for (asset_lock_txid, element) in pending.to_key_elements() {
            let Element::Item(value, _) = element else {
                return Err(Error::Drive(drive::error::Error::Drive(
                    DriveError::CorruptedElementType(
                        "pending asset lock credit inflow is not an item",
                    ),
                )));
            };
            let pending = PendingAssetLockCreditInflow::from_bytes(&value)?;
            if pending
                .recorded_at_core_height
                .saturating_add(window_max_blocks)
                <= chain_locked_height
            {
                self.drive.batch_delete(
                    (&pending_path).into(),
                    &asset_lock_txid,
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    Some(transaction),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        self.drive.apply_batch_low_level_drive_operations(
            None,
            Some(transaction),
            batch_operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;
    use drive::drive::identity::withdrawals::paths::{
        get_withdrawal_credit_inflows_sum_tree_path_vec,
        get_withdrawal_transactions_sum_tree_path_vec,
    };
    use drive::grovedb::{Element, PathQuery, Query, SizedQuery};
    use drive::util::grove_operations::BatchInsertApplyType;
    use drive::util::object_size_info::PathKeyElementInfo;

    /// Both the reserved withdrawal amounts and the credit inflows expire on the same
    /// schedule; the v1 cleanup must prune the entries of both trees whose key is before the
    /// block time and leave the rest.
    #[test]
    fn should_prune_expired_entries_of_both_sum_trees() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transaction = platform.drive.grove.start_transaction();

        let now_ms: u64 = 1_000_000;

        for path in [
            get_withdrawal_transactions_sum_tree_path_vec(),
            get_withdrawal_credit_inflows_sum_tree_path_vec(),
        ] {
            for (key_time_ms, amount) in [(now_ms - 1, 100i64), (now_ms, 250i64)] {
                let mut drive_operations = vec![];
                platform
                    .drive
                    .batch_insert_sum_item_or_add_to_if_already_exists(
                        PathKeyElementInfo::PathKeyElement::<0>((
                            path.clone(),
                            key_time_ms.to_be_bytes().to_vec(),
                            Element::new_sum_item(amount),
                        )),
                        BatchInsertApplyType::StatefulBatchInsert,
                        Some(&transaction),
                        &mut drive_operations,
                        &platform_version.drive,
                    )
                    .expect("expected to insert the entry");
                platform
                    .drive
                    .apply_batch_low_level_drive_operations(
                        None,
                        Some(&transaction),
                        drive_operations,
                        &mut vec![],
                        &platform_version.drive,
                    )
                    .expect("expected to apply the entry");
            }
        }

        platform
            .cleanup_expired_locks_of_withdrawal_amounts_v1(
                &BlockInfo {
                    time_ms: now_ms,
                    height: 100,
                    core_height: 10,
                    epoch: Epoch::default(),
                },
                &transaction,
                platform_version,
            )
            .expect("expected the cleanup to succeed");

        for path in [
            get_withdrawal_transactions_sum_tree_path_vec(),
            get_withdrawal_credit_inflows_sum_tree_path_vec(),
        ] {
            let mut query = Query::new();
            query.insert_all();
            let (results, _) = platform
                .drive
                .grove_get_raw_path_query(
                    &PathQuery::new(path, SizedQuery::new(query, None, None)),
                    Some(&transaction),
                    drive::grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to query the tree");
            let keys: Vec<_> = results
                .to_key_elements()
                .into_iter()
                .map(|(key, _)| key)
                .collect();
            // The entry exactly at the block time is not expired yet (strict `<`).
            assert_eq!(keys, vec![now_ms.to_be_bytes().to_vec()]);
        }
    }

    /// The trees of the Core-anchored withdrawal limit are pruned by Core height: Core-dated
    /// inflows once their Core height is reached, balances older than the band the limit reads,
    /// and asset locks that waited a whole band of Core blocks without Core mining them.
    #[test]
    fn should_prune_the_core_anchored_trees_by_core_height() {
        use drive::drive::identity::withdrawals::paths::{
            get_withdrawal_core_dated_credit_inflows_sum_tree_path_vec,
            get_withdrawal_pending_asset_lock_inflows_path_vec,
        };

        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();
        let block = |core_height: u32| BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height,
            epoch: Epoch::default(),
        };

        // Balances at Core heights 399, 400 and 401.
        for core_height in [399, 400, 401] {
            platform
                .drive
                .record_core_credit_pool_block(
                    core_height,
                    1,
                    &[],
                    &block(core_height),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to record the block");
        }
        // Core-dated inflows mined at 447 and 448: they stop counting at 999 and 1000.
        for (asset_lock, mined_at) in [(1u8, 447u32), (2, 448)] {
            platform
                .drive
                .record_asset_lock_credit_inflow(
                    [asset_lock; 32],
                    10,
                    Some(mined_at),
                    &block(450),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to record the inflow");
        }
        // Pending asset locks recorded at Core heights 400 and 401: dropped at 1000 and 1001.
        for (asset_lock, recorded_at) in [(3u8, 400u32), (4, 401)] {
            platform
                .drive
                .record_asset_lock_credit_inflow(
                    [asset_lock; 32],
                    10,
                    None,
                    &block(recorded_at),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to record the inflow");
        }

        platform
            .cleanup_expired_locks_of_withdrawal_amounts_v1(
                &block(1000),
                &transaction,
                platform_version,
            )
            .expect("expected the cleanup to succeed");

        let keys = |path: Vec<Vec<u8>>| {
            let mut query = Query::new();
            query.insert_all();
            platform
                .drive
                .grove_get_raw_path_query(
                    &PathQuery::new(path, SizedQuery::new(query, None, None)),
                    Some(&transaction),
                    drive::grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to query")
                .0
                .to_key_elements()
                .into_iter()
                .map(|(key, _)| key)
                .collect::<Vec<_>>()
        };

        // The band at 1000 starts at 400: 399 goes.
        assert_eq!(
            platform
                .drive
                .fetch_core_credit_pool_balances(0..=1000, Some(&transaction), platform_version)
                .expect("expected the balances")
                .into_keys()
                .collect::<Vec<_>>(),
            vec![400, 401]
        );
        // The inflow that stops counting at 999 goes, the one at 1000 has just stopped too.
        assert!(keys(get_withdrawal_core_dated_credit_inflows_sum_tree_path_vec()).is_empty());
        // The asset lock pending since 400 waited 600 Core blocks; the one since 401 stays.
        assert_eq!(
            keys(get_withdrawal_pending_asset_lock_inflows_path_vec()),
            vec![vec![4u8; 32]]
        );
    }
}
