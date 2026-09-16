use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use drive::drive::identity::withdrawals::paths::{
    get_withdrawal_credit_inflows_sum_tree_path_vec, get_withdrawal_transactions_sum_tree_path_vec,
};
use drive::grovedb::{MaybeTree, PathQuery, QueryItem, Transaction};
use drive::util::grove_operations::BatchDeleteApplyType;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Version 1 differs from version 0 in also pruning the expired entries of the credit
    /// inflows sum tree, which exists from protocol version 14: both trees are keyed by the
    /// block time their entries stop counting toward the daily withdrawal limit, on the same
    /// 25 hour schedule, and both are pruned with the same per-block limit.
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
}
